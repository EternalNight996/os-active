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

fn default_close_secs() -> u64 {
  3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppCfg {
  /// [app] 表
  #[serde(default)]
  pub app: AppInner,
}
impl Default for AppCfg {
  fn default() -> Self {
    Self {
      app: AppInner::default(),
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

impl AppCfg {
  /// 从运行目录加载 os-active.toml;文件缺失/解析失败时用默认值
  pub fn load() -> Self {
    let path = std::env::current_dir().unwrap_or_default().join(CFG_FNAME);
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
        info!("未找到配置文件 {},使用默认配置(auto_close=false, close_after_secs=3)", path.display());
        Self::default()
      }
    }
  }
}