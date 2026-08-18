//! e-log 日志配置(复刻 etest-core logger.rs,e-log 0.3 tracing 方案)
//!
//! 输出: logs/os-active.log(明细) + stdout;panic 另写 bug.os-active.log
use ::core::fmt;
pub use e_log::*;
use e_log::{
  __private::{Event, Subscriber},
  appender::non_blocking::WorkerGuard,
  subscriber::{
    fmt::{format, FmtContext, FormatEvent, FormatFields, FormattedFields},
    layer::SubscriberExt as _,
    registry::LookupSpan,
    Registry,
  },
};
use e_utils::fs::AutoPath as _;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 日志文件名(用户约定: logs/文件名.log)
pub const LOG_FNAME: &str = "os-active.log";

/// 日志格式: [2026-01-01 12:00:00.123 INFO] 消息
struct MyLogFormat;

impl<S, N> FormatEvent<S, N> for MyLogFormat
where
  S: Subscriber + for<'a> LookupSpan<'a>,
  N: for<'a> FormatFields<'a> + 'static,
{
  fn format_event(&self, ctx: &FmtContext<'_, S, N>, mut writer: format::Writer<'_>, event: &Event<'_>) -> fmt::Result {
    let metadata = event.metadata();
    let now = e_utils::chrono::china_now().unwrap_or_default();
    write!(&mut writer, "[{} {}] {} ", now.format("%Y-%m-%d"), now.format("%H:%M:%S%.3f"), metadata.level())?;
    if let Some(scope) = ctx.event_scope() {
      for span in scope.from_root() {
        write!(writer, "{}", span.name())?;
        let ext = span.extensions();
        let fields = &ext.get::<FormattedFields<N>>().expect("will never be `None`");
        if !fields.is_empty() {
          write!(writer, "{{{}}}", fields)?;
        }
        write!(writer, ": ")?;
      }
    }
    ctx.field_format().format_fields(writer.by_ref(), event)?;
    writeln!(writer)
  }
}

#[derive(Deserialize, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogConf {
  pub level: Level,
  folder: PathBuf,
  fname: String,
  pub format: String,
  pub output_list: Vec<String>,
  pub tracing: bool,
}
impl Default for LogConf {
  fn default() -> Self {
    Self {
      level: Level::Debug,
      // 与 etest-core ORIGIN 语义一致:当前工作目录下 logs/
      folder: std::env::current_dir().unwrap_or_default().join("logs"),
      fname: LOG_FNAME.to_string(),
      format: String::new(),
      output_list: vec![],
      tracing: false,
    }
  }
}
impl LogConf {
  /// 日志文件名
  pub fn get_fname(&self) -> String {
    self.fname.clone()
  }
  /// 日志目录
  pub fn get_folder(&self) -> PathBuf {
    self.folder.clone()
  }
  /// 初始化:建目录 + 挂 panic hook + 装载 subscriber
  pub fn init(&self, sub: impl Subscriber + Send + Sync) -> e_utils::AnyResult<()> {
    self.folder.auto_create_dir()?;
    e_log::panic::reattach_windows_terminal();
    e_log::panic::set_panic_hook(&self.folder, &format!("bug.{}", self.fname))?;
    e_log::core::init_subscriber(sub, self.tracing);
    Ok(())
  }
  /// 组合订阅器:文件层(永久滚动, no ANSI) + stdout 层
  /// 返回的 WorkerGuard 必须存活于整个进程生命周期,否则日志线程被回收
  pub fn get_subscriber(&self, level: Level) -> (impl Subscriber + Send + Sync, Vec<WorkerGuard>) {
    let roll = appender::rolling::never(&self.folder, &self.fname, FileShare::Read);
    let (f, guard) = appender::non_blocking(roll);
    let file_layer = subscriber::fmt::layer()
      .without_time()
      .with_ansi(false)
      .event_format(MyLogFormat)
      .with_writer(f);
    let (f2, guard2) = appender::non_blocking(std::io::stdout());
    let base_layer = subscriber::fmt::layer()
      .without_time()
      .with_ansi(false)
      .event_format(MyLogFormat)
      .with_writer(f2);
    let def = Registry::default()
      .with(level.to_level_filter())
      .with(base_layer)
      .with(file_layer);
    (def, vec![guard, guard2])
  }
}