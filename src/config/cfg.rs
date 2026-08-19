//! 应用配置表(os-active.toml,放在运行目录;缺失时用默认值)
//!
//! 示例:
//! ```toml
//! [app]
//! close_after_secs = 3    # 激活成功并确认后,倒计时 N 秒自动关闭窗口
//! auto_close = false      # true:激活成功自动确认并倒计时关闭(无需人工点确认)
//! ```
use e_log::preload::*;
use serde::{Deserialize, Serialize};

/// 配置文件路径(运行目录)
pub const CFG_FNAME: &str = "os-active.toml";

/// 默认配置表内容(自动生成时写入,含注释)
const DEFAULT_CFG: &str = "# ============================================================\n# os-active 配置表(首次运行自动生成,修改后重启生效)\n# ============================================================\n[app]\n# PASS(激活成功并确认)后倒计时 N 秒自动关闭窗口\nclose_after_secs = 3\n# true:激活成功后自动确认并倒计时关闭,无需人工点击确认按钮\nauto_close = false\n\n[sn]\n# SN 工具(ByoDmi)路径,留空自动探测;可填绝对路径或相对 tools/ 的路径\ntool_path = \"\"\n# 是否校验 SN 不为空(空则日志告警)\nrequire_sn = true\n
";

fn default_close_secs() -> u64 {
  3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppCfg {
  /// [app] 表
  #[serde(default)]
  pub app: AppInner,
  /// [sn] 表(SN 读取配置)
  #[serde(default)]
  pub sn: SnInner,
}
impl Default for AppCfg {
  fn default() -> Self {
    Self {
      app: AppInner::default(),
      sn: SnInner::default(),
    }
  }
}

/// [app] 表配置项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInner {
  /// PASS 后倒计时秒数,到时自动关闭窗口
  #[serde(default = "default_close_secs")]
  pub close_after_secs: u64,
  /// true:激活成功后自动确认并倒计时关闭(无需人工点确认按钮)
  #[serde(default)]
  pub auto_close: bool,
}
impl Default for AppInner {
  fn default() -> Self {
    Self {
      close_after_secs: default_close_secs(),
      auto_close: false,
    }
  }
}

/// [sn] 表配置项(SN 读取)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnInner {
  /// SN 工具(ByoDmi)路径;留空则自动探测 tools/<工具>/<架构>/<os>/<工具>
  #[serde(default)]
  pub tool_path: Option<String>,
  /// 是否校验 SN 不为空(空则日志告警)
  #[serde(default = "default_require_sn")]
  pub require_sn: bool,
}
fn default_require_sn() -> bool {
  true
}
impl Default for SnInner {
  fn default() -> Self {
    Self {
      tool_path: None,
      require_sn: true,
    }
  }
}

impl AppCfg {
  /// 从运行目录加载 os-active.toml;文件缺失时自动生成默认配置表,解析失败时用默认值
  pub fn load() -> Self {
    let path = std::env::current_dir().unwrap_or_default().join(CFG_FNAME);
    if !path.exists() {
      // 自动生成默认配置表(含注释说明),便于用户查看可配置项
      let _ = std::fs::write(&path, DEFAULT_CFG);
      info!("未找到配置文件,已自动生成默认配置表: {}", path.display());
    }
    match std::fs::read_to_string(&path) {
      Ok(s) => match toml::from_str::<Self>(&s) {
        Ok(cfg) => {
          info!("加载配置: {}(auto_close={}, close_after_secs={})", path.display(), cfg.app.auto_close, cfg.app.close_after_secs);
          cfg
        }
        Err(e) => {
          warn!("配置文件解析失败({}),使用默认配置: {e}", path.display());
          Self::default()
        }
      },
      Err(_) => {
        warn!("配置文件读取失败,使用默认配置");
        Self::default()
      }
    }
  }
}