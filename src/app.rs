//! 应用主界面(egui)
//!
//! 流程状态机:
//!   Detect(检测中) -> Confirm(已激活,等待确认) -> Countdown(倒计时) -> Closing(关闭)
//!   - 已激活 + 人工点「确认激活状态」或配置 auto_close=true -> 进入倒计时 -> 关闭
//!   - 退出时(on_exit)输出 ETest 标准 R<{json}>R;仅 已激活+已确认 才 status:true
//! 检测在后台线程执行,完成后经 channel 回投 UI 线程。
use std::{
  path::PathBuf,
  sync::mpsc,
  time::{Duration, Instant},
};

use e_log::appender::non_blocking::WorkerGuard;
use e_log::preload::*;
use eframe::egui::{self, Color32, RichText};

use crate::config::{cargo, cfg::AppCfg, logger::LogConf};
use crate::detect::{
  self,
  model::{Activation, DetectResult},
};

/// 界面阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
  /// 检测中/等待检测完成
  Detect,
  /// 检测完成且已激活,等待人工确认
  Confirm,
  /// 检测完成但未激活(auto_close=false 时),等待重新检测/关闭
  FailWait,
  /// 已确认,倒计时中(剩余秒数)
  Countdown { left: u64 },
  /// 已发关闭命令,等待退出
  Closing,
}

/// 状态主色
fn activation_color(a: Activation) -> Color32 {
  match a {
    Activation::Activated => Color32::from_rgb(0x1f, 0xa0, 0x4f), // 绿
    Activation::NotActivated => Color32::from_rgb(0xd0, 0x33, 0x33), // 红
    Activation::NotApplicable => Color32::from_rgb(0x1e, 0x6f, 0xb8), // 蓝
    Activation::Unknown => Color32::from_rgb(0x8a, 0x8a, 0x8a), // 灰
  }
}

pub struct App {
  ctx: egui::Context,
  /// 日志守护(进程生命周期内保持存活,勿 drop)
  _guards: Vec<WorkerGuard>,
  /// 应用配置(os-active.toml)
  cfg: AppCfg,
  result: Option<DetectResult>,
  checking: bool,
  rx: mpsc::Receiver<DetectResult>,
  tx: mpsc::Sender<DetectResult>,
  log_path: PathBuf,
  /// 日志文件尾部(展示用)
  log_tail: Vec<String>,
  /// 流程阶段
  phase: Phase,
  /// 是否已确认激活状态(人工点按钮或 auto_close)
  confirmed: bool,
  /// 确认方式:button / auto
  confirm_by: &'static str,
  /// 倒计时 tick 基准
  last_tick: Instant,
  /// R 结果是否已输出(退出只输出一次)
  r_emitted: bool,
}

impl App {
  pub fn new(ctx: egui::Context, guards: Vec<WorkerGuard>) -> Self {
    let (tx, rx) = mpsc::channel::<DetectResult>();
    let mut app = Self {
      ctx: ctx.clone(),
      _guards: guards,
      cfg: AppCfg::load(),
      result: None,
      checking: false,
      rx,
      tx,
      log_path: sn_log_path(),
      log_tail: vec![],
      phase: Phase::Detect,
      confirmed: false,
      confirm_by: "button",
      last_tick: Instant::now(),
      r_emitted: false,
    };
    app.trigger_detect();
    app
  }

  /// 启动一次后台检测
  fn trigger_detect(&mut self) {
    if self.checking {
      return;
    }
    self.checking = true;
    self.phase = Phase::Detect;
    self.result = None;
    info!("触发重新检测");
    let tx = self.tx.clone();
    std::thread::spawn(move || {
      let r = detect::run();
      let _ = tx.send(r);
    });
    self.ctx.request_repaint_after(Duration::from_millis(100));
  }

  /// 收结果 + 刷新日志尾部
  fn poll(&mut self) {
    while let Ok(r) = self.rx.try_recv() {
      info!("检测完成: {} -> {}", r.activation.label(), r.summary);
      self.result = Some(r);
      self.checking = false;
      self.log_tail = read_log_tail(&self.log_path, 15);
    }
    if self.checking {
      self.ctx.request_repaint_after(Duration::from_millis(100));
    }
  }

  // ------------------------------------------------ 确认/倒计时状态机

  /// 检测完成后:已激活则进入等待确认(auto_close 则自动确认),其余状态等待手动关窗
  fn advance_after_detect(&mut self) {
    if !matches!(self.phase, Phase::Detect) {
      return;
    }
    let Some(r) = &self.result else {
      return;
    };
    if r.activation != Activation::Activated {
      // FAIL(未激活等):auto_close=true -> 自动倒计时关闭;否则进入 FailWait 等待重新检测
      if self.cfg.app.auto_close {
        info!("auto_close=true: 未激活({}),自动倒计时关闭窗口", r.activation.label());
        self.enter_countdown();
      } else {
        self.phase = Phase::FailWait;
        info!("未激活({}),请点击重新检测或关闭窗口", r.activation.label());
      }
      return;
    }
    if self.cfg.app.auto_close {
      info!("自动确认(auto_close=true): 激活状态已确认,进入倒计时");
      self.confirmed = true;
      self.confirm_by = "auto";
      self.enter_countdown();
    } else {
      self.phase = Phase::Confirm;
      info!("激活成功,等待人工确认(点击「确认激活状态」按钮)");
    }
  }

  /// 人工确认(按钮点击)
  fn confirm_by_button(&mut self) {
    if self.confirmed {
      return;
    }
    info!("人工确认通过: 用户点击确认按钮,激活状态已确认");
    self.confirmed = true;
    self.confirm_by = "button";
    self.enter_countdown();
  }

  /// 进入倒计时
  fn enter_countdown(&mut self) {
    let left = self.cfg.app.close_after_secs.max(1);
    info!("开始倒计时 {} 秒,到时自动关闭窗口并输出结果", left);
    self.phase = Phase::Countdown { left };
    self.last_tick = Instant::now();
    self.ctx.request_repaint_after(Duration::from_millis(100));
  }

  /// 每秒 tick 倒计时;归零后关闭窗口(on_exit 输出 R 结果)
  fn tick_countdown(&mut self) {
    let Phase::Countdown { left } = self.phase else {
      return;
    };
    if self.last_tick.elapsed() < Duration::from_secs(1) {
      self.ctx.request_repaint_after(Duration::from_millis(100));
      return;
    }
    self.last_tick = Instant::now();
    if left <= 1 {
      info!("倒计时结束,关闭窗口(输出 R 结果)");
      self.phase = Phase::Closing;
      self.ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    } else {
      self.phase = Phase::Countdown { left: left - 1 };
    }
  }

  /// 输出 ETest 标准 R<{json}>R(仅一次)
  /// status=true 需同时满足: 1)激活校验通过(Activated) 2)已确认(按钮或 auto_close)
  fn emit_r(&mut self) {
    if self.r_emitted {
      return;
    }
    self.r_emitted = true;
    let (status, content, opts) = match &self.result {
      Some(r) if r.activation == Activation::Activated && self.confirmed => (
        true,
        format!("PASS;{};{}确认", r.summary, self.confirm_by),
        build_opts(r, true, self.confirm_by),
      ),
      Some(r) if r.activation == Activation::Activated => (
        false,
        "NG;激活成功但未确认(需点击确认按钮或配置 auto_close=true)".to_string(),
        build_opts(r, false, "none"),
      ),
      Some(r) => (
        false,
        format!("NG;{}", r.summary),
        build_opts(r, false, "none"),
      ),
      None => (
        false,
        "NG;未完成检测".to_string(),
        serde_json::json!({ "activation": "未知", "os": "", "summary": "未完成检测" }),
      ),
    };
    let json = serde_json::json!({
      "content": content,
      "status": status,
      "opts": opts,
    });
    let out = format!("R<{}>R", json);
    info!("输出结果: {out}");
    // ETest 主程序捕获 stdout 解析 R<...>R,必须 flush
    println!("{out}");
    use std::io::Write;
    let _ = std::io::stdout().flush();
    self.log_tail = read_log_tail(&self.log_path, 15);
  }

  // ------------------------------------------------------------- UI

  fn status_hero(&self, ui: &mut egui::Ui) {
    let (text, color) = match &self.result {
      Some(r) => (r.activation.label().to_string(), activation_color(r.activation)),
      None => ("检测中...".to_string(), Color32::from_rgb(0x8a, 0x8a, 0x8a)),
    };
    ui.vertical_centered(|ui| {
      ui.label(RichText::new(text).size(56.0).strong().color(color));
      if let Some(r) = &self.result {
        ui.label(RichText::new(&r.summary).size(18.0).color(Color32::from_gray(120)));
        ui.label(RichText::new(format!("检测时间: {}", r.checked_at)).size(13.0).color(Color32::from_gray(110)));
      }
    });
  }

  /// 底部栏:确认按钮(颜色随激活状态变化) + 倒计时进度
  fn bottom_bar(&mut self, ui: &mut egui::Ui) {
    egui::Panel::bottom("bottom_bar").show(ui, |ui| {
      ui.add_space(8.0);
      ui.vertical_centered(|ui| {
        match self.phase {
          Phase::Confirm => {
            // 已激活待确认:绿色按钮
            if ui
              .add_sized(
                [260.0, 46.0],
                egui::Button::new(RichText::new("确认激活状态").size(17.0).strong().color(Color32::WHITE))
                  .fill(Color32::from_rgb(0x1f, 0xa0, 0x4f)),
              )
              .clicked()
            {
              self.confirm_by_button();
            }
          }
          Phase::FailWait => {
            // 未激活:状态文字 + 重新检测按钮
            let (text, color) = match &self.result {
              Some(r) => (
                r.activation.label(),
                match r.activation {
                  Activation::NotActivated => Color32::from_rgb(0xd0, 0x33, 0x33),
                  Activation::NotApplicable => Color32::from_rgb(0x1e, 0x6f, 0xb8),
                  _ => Color32::from_rgb(0x8a, 0x8a, 0x8a),
                },
              ),
              None => ("检测中...", Color32::from_rgb(0x8a, 0x8a, 0x8a)),
            };
            ui.label(RichText::new(text).size(17.0).strong().color(color));
            if ui
              .add_sized([200.0, 40.0], egui::Button::new(RichText::new("重新检测").size(16.0).strong()))
              .clicked()
            {
              self.trigger_detect();
            }
          }
          Phase::Countdown { left } => {
            let total = self.cfg.app.close_after_secs.max(1) as f32;
            let done = (total - left as f32) / total;
            ui.label(
              RichText::new(format!("已确认({}),{} 秒后自动关闭窗口...", self.confirm_by, left))
                .size(15.0)
                .strong()
                .color(Color32::from_rgb(0x1f, 0xa0, 0x4f)),
            );
            ui.add(egui::ProgressBar::new(done).desired_width(300.0));
          }
          Phase::Closing => {
            ui.label(RichText::new("正在关闭...").size(15.0).color(Color32::from_gray(120)));
          }
          Phase::Detect => {
            // 颜色随激活状态:绿=已激活(可确认) 红=未激活 蓝=无需激活 灰=检测中/无法判定
            let (text, color, enabled) = match &self.result {
              Some(r) if r.activation == Activation::Activated => {
                ("确认激活状态", Color32::from_rgb(0x1f, 0xa0, 0x4f), true)
              }
              Some(r) => match r.activation {
                Activation::NotActivated => ("未激活", Color32::from_rgb(0xd0, 0x33, 0x33), false),
                Activation::NotApplicable => ("无需激活", Color32::from_rgb(0x1e, 0x6f, 0xb8), false),
                Activation::Unknown => ("无法判定", Color32::from_rgb(0x8a, 0x8a, 0x8a), false),
                Activation::Activated => ("确认激活状态", Color32::from_rgb(0x1f, 0xa0, 0x4f), true),
              },
              None => ("检测中...", Color32::from_rgb(0x8a, 0x8a, 0x8a), false),
            };
            let resp = ui.add_sized(
              [260.0, 46.0],
              egui::Button::new(RichText::new(text).size(17.0).strong().color(Color32::WHITE)).fill(color),
            );
            if resp.clicked() && enabled {
              self.confirm_by_button();
            }
            if !enabled {
              resp.on_hover_text("未通过激活校验,不可确认");
            }
          }
        }
      });
      ui.add_space(8.0);
    });
  }
  fn sys_info(&self, ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.label(RichText::new("系统信息").size(16.0).strong());
    egui::Grid::new("sys_info").num_columns(4).spacing([16.0, 6.0]).show(ui, |ui| {
      if let Some(r) = &self.result {
        ui.label("系统:");
        ui.label(&r.os.name);
        ui.label("版本:");
        ui.label(&r.os.version);
        ui.end_row();
        ui.label("架构:");
        ui.label(&r.os.arch);
        ui.label("发行版 ID:");
        ui.label(&r.os.distro_id);
        ui.end_row();
        if !r.os.sn.is_empty() {
          ui.label("SN:");
          ui.label(RichText::new(r.os.sn.as_str()).color(Color32::from_rgb(0x2f, 0x6f, 0xcf)));
          ui.label("");
          ui.label("");
          ui.end_row();
        }
        if !r.os.pretty.is_empty() {
          ui.label("PRETTY_NAME:");
          ui.label(&r.os.pretty);
          ui.label("");
          ui.label("");
          ui.end_row();
        }
        if let Some(e) = &r.expire_at {
          ui.label("授权到期:");
          ui.label(RichText::new(e.as_str()).color(Color32::from_rgb(0xd0, 0x7a, 0x1f)));
          ui.label("");
          ui.label("");
          ui.end_row();
        }
      } else {
        ui.label("检测中...");
        ui.end_row();
      }
    });
  }

  fn detail_table(&self, ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.label(RichText::new("检测明细").size(16.0).strong());
    let Some(r) = &self.result else {
      ui.label("等待检测结果...");
      return;
    };
    if r.items.is_empty() {
      ui.label("无检测项");
      return;
    }
    egui::ScrollArea::vertical()
      .id_salt("detail_scroll")
      .max_height(200.0)
      .show(ui, |ui| {
        egui::Grid::new("detail_grid")
          .num_columns(4)
          .striped(true)
          .spacing([12.0, 6.0])
          .min_col_width(90.0)
          .show(ui, |ui| {
            ui.label(RichText::new("检测项").strong());
            ui.label(RichText::new("命令").strong());
            ui.label(RichText::new("判定").strong());
            ui.label(RichText::new("输出").strong());
            ui.end_row();
            for it in &r.items {
              ui.label(&it.name);
              ui.label(RichText::new(&it.command).monospace().size(12.0));
              let vcolor = if it.success {
                Color32::from_gray(150)
              } else {
                Color32::from_rgb(0xd0, 0x33, 0x33)
              };
              ui.label(RichText::new(&it.verdict).color(vcolor));
              let out_short = truncate(&it.output, 160);
              let resp = ui.add(
                egui::Label::new(RichText::new(&out_short).monospace().size(12.0)).truncate(),
              );
              if !it.output.is_empty() {
                resp.on_hover_text(it.output.as_str());
              }
              ui.end_row();
            }
          });
      });
  }

  fn log_panel(&mut self, ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
      ui.label(RichText::new("日志").size(16.0).strong());
      if ui.button("打开日志目录").clicked() {
        open_folder(&self.log_path);
      }
      ui.label(RichText::new(self.log_path.display().to_string()).size(12.0).color(Color32::from_gray(120)));
    });
    egui::ScrollArea::vertical()
      .id_salt("log_scroll")
      .max_height(100.0)
      .show(ui, |ui| {
        if self.log_tail.is_empty() {
          ui.label("(日志写入中...)");
        }
        for line in &self.log_tail {
          ui.label(RichText::new(line.as_str()).monospace().size(12.0).color(Color32::from_gray(140)));
        }
      });
  }
}

impl eframe::App for App {
  fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    self.poll();
    self.advance_after_detect();
    self.tick_countdown();
    self.bottom_bar(ui);

    egui::CentralPanel::default().show(ui, |ui| {
      // 标题栏
      ui.horizontal(|ui| {
        ui.label(RichText::new("OS Active").size(22.0).strong().color(Color32::from_rgb(0x2f, 0x6f, 0xcf)));
        ui.label(RichText::new(cargo::get_descript_version()).size(14.0).color(Color32::from_gray(130)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
          let txt = if self.checking { "检测中..." } else { "重新检测" };
          if ui.button(RichText::new(txt).size(15.0)).clicked() && !self.checking {
            self.trigger_detect();
          }
        });
      });
      ui.separator();

      self.status_hero(ui);
      ui.separator();
      self.sys_info(ui);
      ui.separator();
      self.detail_table(ui);
      ui.separator();
      self.log_panel(ui);
    });
  }

  /// 退出时输出 ETest 标准结果(仅一次)
  fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
    self.emit_r();
  }
}

/// 构建 R 结果 opts 明细
fn build_opts(r: &DetectResult, confirmed: bool, confirm_by: &str) -> serde_json::Value {
  let details: Vec<serde_json::Value> = r
    .items
    .iter()
    .map(|it| {
      serde_json::json!({
        "name": it.name,
        "command": it.command,
        "success": it.success,
        "verdict": it.verdict,
      })
    })
    .collect();
  serde_json::json!({
    "activation": r.activation.label(),
    "os": format!("{} {}", r.os.name, r.os.version),
    "sn": r.os.sn,
    "distro": r.os.distro_id,
    "arch": r.os.arch,
    "summary": r.summary,
    "expire_at": r.expire_at,
    "confirmed": confirmed,
    "confirm_by": confirm_by,
    "checked_at": r.checked_at,
    "details": details,
  })
}


/// 构造日志完整路径:logs/<SN>.log(SN 空则 os-active.log)
fn sn_log_path() -> std::path::PathBuf {
  let mut lc = LogConf::default();
  if let Some(sn) = crate::detect::sys::get_sn() {
    lc.set_fname(format!("{sn}-{}.log", crate::config::cargo::NAME));
  }
  lc.get_folder().join(lc.get_fname())
}

/// 读取日志文件尾部 N 行
fn read_log_tail(path: &PathBuf, n: usize) -> Vec<String> {
  let Ok(content) = std::fs::read_to_string(path) else {
    return vec![];
  };
  content.lines().rev().take(n).map(|s| s.to_string()).collect::<Vec<_>>().into_iter().rev().collect()
}

/// 截断字符串(超长加省略号)
fn truncate(s: &str, max: usize) -> String {
  if s.chars().count() <= max {
    s.to_string()
  } else {
    let mut out: String = s.chars().take(max).collect();
    out.push_str("...");
    out
  }
}

/// 打开日志所在文件夹(跨平台)
fn open_folder(path: &PathBuf) {
  let folder = path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
  #[cfg(windows)]
  {
    let _ = e_utils::cmd::Cmd::new("explorer").args([folder.as_str()]).spawn();
  }
  #[cfg(target_os = "linux")]
  {
    let _ = e_utils::cmd::Cmd::new("xdg-open").args([folder.as_str()]).spawn();
  }
  #[allow(unreachable_code)]
  {
    let _ = folder;
  }
}