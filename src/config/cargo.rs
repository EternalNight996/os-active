//! 编译期包信息(对齐 etest config/cargo.rs)
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
#[allow(dead_code)]
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
/// 窗口标题:名称 + 版本
pub fn get_descript_version() -> String {
  format!("{NAME} {VERSION}")
}