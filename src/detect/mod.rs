//! 检测模块:系统识别 + 激活状态探测 + 结果模型
pub mod active;
pub mod model;
pub mod sys;

use e_log::preload::*;

/// 执行一次完整检测(在后台线程调用)
pub fn run() -> model::DetectResult {
  let (sn, sn_probes, sn_source) = sys::probe_sn();
  let mut os = sys::detect_os();
  os.sn = sn.unwrap_or_default();
  os.sn_source = sn_source.clone();
  info!("系统识别: {} {} ({}) CPU={} SN={} 来源={}", os.name, os.version, os.arch, os.cpu, os.sn, os.sn_source);
  let (activation, mut items, summary, expire_at) = active::check(&os);
  // SN 探测明细置前(获取失败时逐项体现)
  items.splice(0..0, sn_probes);
  // 明细逐条落日志(日志体现明细)
  for it in &items {
    info!(
      "[{}] cmd={} success={} verdict={}\n{}",
      it.name, it.command, it.success, it.verdict, it.output
    );
  }
  if let Some(e) = &expire_at {
    info!("授权到期时间: {e}");
  }
  info!("检测结论: {} -> {}", activation.label(), summary);
  model::DetectResult {
    os,
    activation,
    summary,
    items,
    expire_at,
    checked_at: format_now(),
  }
}

/// 本地时间字符串 %Y-%m-%d %H:%M:%S(UTC+8,检测工具无需精确时区)
pub fn format_now() -> String {
  use std::time::{SystemTime, UNIX_EPOCH};
  let secs = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|d| d.as_secs() as i64)
    .unwrap_or(0)
    + 8 * 3600;
  let days = secs.div_euclid(86400);
  let rem = secs.rem_euclid(86400);
  let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
  let (y, mo, d) = civil_from_days(days);
  format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02}:{s:02}")
}

/// days 转公历(Howard Hinnant 算法)
fn civil_from_days(z: i64) -> (i64, i64, i64) {
  let z = z + 719468;
  let era = if z >= 0 { z } else { z - 146096 } / 146097;
  let doe = (z - era * 146097) as u64;
  let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
  let y = yoe as i64 + era * 400;
  let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
  let mp = (5 * doy + 2) / 153;
  let d = doy - (153 * mp + 2) / 5 + 1;
  let m = if mp < 10 { mp + 3 } else { mp - 9 };
  (if m <= 2 { y + 1 } else { y }, m as i64, d as i64)
}