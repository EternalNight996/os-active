#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod data;
mod detect;

use config::logger::LogConf;
use e_log::preload::*;

fn main() -> eframe::Result<()> {
  // 初始化 e-log:logs/<SN>.log + stdout + panic hook(获取不到 SN 时用 os-active.log)
  let mut log = LogConf::default();
  // logs/<SN>-os-active.log(SN+项目名);获取不到 SN 用 os-active.log
  if let Some(sn) = detect::sys::get_sn() {
    log.set_fname(format!("{sn}-{}.log", config::cargo::NAME));
  }
  let (sub, guards) = log.get_subscriber(log.level);
  log.init(sub).expect("初始化日志失败");
  info!("{} 启动", config::cargo::get_descript_version());
  if let Some(sn) = detect::sys::get_sn() {
    info!("设备 SN: {sn}");
  }

  let options = eframe::NativeOptions {
    renderer: eframe::Renderer::Glow, // 对齐 etest:显式 Glow(eframe 0.35+ 默认 wgpu)
    viewport: eframe::egui::ViewportBuilder::default()
      .with_inner_size([880.0, 760.0])
      .with_min_inner_size([720.0, 600.0])
      .with_title("OS Active 系统激活状态检测"),
    ..Default::default()
  };
  let res = eframe::run_native(
    "os-active",
    options,
    Box::new(move |cc| {
      data::font::load(&cc.egui_ctx);
      Ok(Box::new(app::App::new(cc.egui_ctx.clone(), guards)))
    }),
  );
  if let Err(e) = &res {
    error!("run_native 返回错误: {e}");
  }
  res
}