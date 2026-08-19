//! 激活状态检测(按平台分发)
//!
//! Windows: slmgr.vbs(-xpr 到期状态 / -dlv 授权详情)
//! 银河麒麟 V10: licence -l(授权列表) + /etc/.kyinfo 授权文件
//! 统信 UOS V20: uos-activator-cmd -q(激活状态)
//! Ubuntu: 无激活机制 -> 无需激活
use super::model::{Activation, CheckItem, OsInfo};
use e_utils::cmd::Cmd;

/// 按系统执行激活检测
#[allow(unused_variables)]
pub fn check(os: &OsInfo) -> (Activation, Vec<CheckItem>, String, Option<String>) {
  #[cfg(windows)]
  {
    check_windows()
  }
  #[cfg(target_os = "linux")]
  {
    check_linux(os)
  }
  #[cfg(not(any(target_os = "linux", windows)))]
  {
    (
      Activation::Unknown,
      vec![CheckItem::new("平台支持", "n/a", false, "当前平台不支持激活检测".into(), "不支持的平台")],
      "当前平台不支持激活检测".to_string(),
      None,
    )
  }
}

/// 执行命令并记录明细(输出编码由 e-utils auto_decode 处理 GBK/UTF-8)
/// cwd: 可选工作目录(Windows 的 cscript 需在 System32 下找 slmgr.vbs)
fn run_check(name: &str, cmd: &str, args: &[&str], cwd: Option<&str>) -> CheckItem {
  let command_line = if args.is_empty() {
    cmd.to_string()
  } else {
    format!("{} {}", cmd, args.join(" "))
  };
  let mut c = Cmd::new(cmd).args(args.iter().copied());
  if let Some(d) = cwd {
    c = c.cwd(d);
  }
  match c.output() {
    Ok(o) => CheckItem::new(name, &command_line, true, o.stdout, "命令执行成功"),
    Err(e) => CheckItem::new(name, &command_line, false, format!("{e}"), "命令执行失败(可能未安装)"),
  }
}

/// 读取文件并记录明细(用于授权文件探测)
#[cfg(target_os = "linux")]
fn read_file_check(path: &str, name: &str) -> CheckItem {
  match std::fs::read_to_string(path) {
    Ok(c) => CheckItem::new(name, &format!("cat {path}"), true, c, "文件存在"),
    Err(e) => CheckItem::new(name, &format!("cat {path}"), false, format!("{path}: {e}"), "文件不存在或不可读"),
  }
}

/// 关键字判定(未激活优先,避免 not activated 误命中 activated)
#[cfg(target_os = "linux")]
fn judge(out: &str, act_kw: &[&str], not_kw: &[&str]) -> Activation {
  let s = out.to_lowercase();
  for kw in not_kw {
    if s.contains(kw) {
      return Activation::NotActivated;
    }
  }
  for kw in act_kw {
    if s.contains(kw) {
      return Activation::Activated;
    }
  }
  Activation::Unknown
}

// ---------------------------------------------------------------- Windows

#[cfg(windows)]
fn check_windows() -> (Activation, Vec<CheckItem>, String, Option<String>) {
  // slmgr.vbs 位于 System32,必须指定工作目录否则 cscript 找不到脚本
  let sys32 = std::env::var("SystemRoot")
    .unwrap_or_else(|_| "C:\\Windows".to_string())
    + "\\System32";
  // 1) 激活到期状态(-xpr):中文/英文系统输出不同
  let xpr = run_check("激活状态查询", "cscript", &["/nologo", "slmgr.vbs", "-xpr"], Some(&sys32));
  // 2) 详细授权信息(-dlv):含 LicenseStatus / 产品名 / 授权状态
  let dlv = run_check("详细授权信息", "cscript", &["/nologo", "slmgr.vbs", "-dlv"], Some(&sys32));
  let (xpr_out, dlv_out) = (xpr.output.clone(), dlv.output.clone());
  let items = vec![xpr, dlv];

  let act = if items.iter().all(|i| !i.success) {
    Activation::Unknown
  } else {
    judge_windows(&xpr_out, &dlv_out)
  };
  let summary = match act {
    Activation::Activated => "Windows 已激活".to_string(),
    Activation::NotActivated => "Windows 未激活(通知模式)".to_string(),
    Activation::Unknown => "Windows 激活状态无法判定,请查看下方明细".to_string(),
    Activation::NotApplicable => "无需激活".to_string(),
  };
  // 提取批量激活到期时间(-xpr 输出:批量激活将于 2027/2/4 17:50:22 过期 / 永久激活)
  let expire = extract_date(&xpr_out);
  (act, items, summary, expire)
}

#[cfg(windows)]
fn judge_windows(xpr: &str, dlv: &str) -> Activation {
  let x = xpr.to_lowercase();
  let d = dlv.to_lowercase();
  // 未激活优先
  if x.contains("通知模式")
    || x.contains("未激活")
    || x.contains("notification")
    || x.contains("not activated")
    || d.contains("unlicensed")
    || d.contains("未授权")
    || d.contains("未激活")
  {
    return Activation::NotActivated;
  }
  // 已激活:永久 / 已激活 / 限时到期(将在...到期) / licensed
  if x.contains("永久激活")
    || x.contains("已激活")
    || x.contains("permanently activated")
    || x.contains("activated")
    || x.contains("将在")
    || x.contains("到期")
    || x.contains("过期")
    || x.contains("expires")
    || d.contains("licensed")
    || d.contains("已授权")
  {
    return Activation::Activated;
  }
  Activation::Unknown
}

// ------------------------------------------------------------------ Linux

#[cfg(target_os = "linux")]
fn check_linux(os: &OsInfo) -> (Activation, Vec<CheckItem>, String, Option<String>) {
  let mut items: Vec<CheckItem> = vec![];
  match os.distro_id.as_str() {
    "ubuntu" => {
      items.push(CheckItem::new("激活机制", "n/a", true, "Ubuntu 无系统激活机制,安装即完整授权".into(), "无需激活"));
      (Activation::NotApplicable, items, "Ubuntu 无需激活".to_string(), None)
    }
    "kylin" => check_kylin(&mut items),
    "uos" | "deepin" => check_uos(&mut items),
    // 未知 ID:存在麒麟授权文件则按麒麟处理
    _ if std::path::Path::new("/etc/.kyinfo").exists() => check_kylin(&mut items),
    _ => {
      items.push(CheckItem::new(
        "系统识别",
        "cat /etc/os-release",
        true,
        format!("未识别的发行版: {}", os.pretty),
        "未知系统",
      ));
      (Activation::Unknown, items, "无法识别的系统发行版".to_string(), None)
    }
  }
}

/// 银河麒麟 V10: licence -l + /etc/.kyinfo
#[cfg(target_os = "linux")]
fn check_kylin(items: &mut Vec<CheckItem>) -> (Activation, Vec<CheckItem>, String, Option<String>) {
  let lic = run_check("麒麟授权查询", "licence", &["-l"], None);
  let kyinfo = read_file_check("/etc/.kyinfo", "麒麟授权文件(.kyinfo)");
  let (lic_success, lic_out) = (lic.success, lic.output.clone());
  let (kyinfo_success, kyinfo_out) = (kyinfo.success, kyinfo.output.clone());
  items.push(lic);
  items.push(kyinfo);

  let act = if lic_success {
    judge(
      &lic_out,
      &["已激活", "activated", "永久授权", "授权成功", "已授权"],
      &["未激活", "not activated", "试用", "trial", "invalid", "expired", "未授权", "未激活"],
    )
  } else if kyinfo_success {
    // licence 命令不可用时解析 /etc/.kyinfo 的授权到期时间(to=term=YYYY-MM-DD):
    //   term >= 今天 -> 已激活(有效期内);term < 今天 -> 未激活(已过期);
    //   解析不出 term -> 无法判定(.kyinfo 存在不代表已激活,过期授权文件也会留存)
    match kyinfo_expire_days(&kyinfo_out) {
      Some(expire) if expire >= today_days() => Activation::Activated,
      Some(_) => Activation::NotActivated,
      None => Activation::Unknown,
    }
  } else {
    Activation::Unknown
  };
  // 授权到期时间(.kyinfo term 字段,官方:term=到期期限)
  let expire = extract_kyinfo_term(&kyinfo_out);
  let summary = match act {
    Activation::Activated => "银河麒麟已激活".to_string(),
    Activation::NotActivated => "银河麒麟未激活".to_string(),
    Activation::Unknown => "银河麒麟激活状态无法判定,请查看下方明细".to_string(),
    Activation::NotApplicable => "无需激活".to_string(),
  };
  (act, items.clone(), summary, expire)
}


/// 解析 .kyinfo 中的授权到期时间(to=term=YYYY-MM-DD,支持 - / . 分隔),返回绝对天数
#[allow(dead_code)]
fn kyinfo_expire_days(content: &str) -> Option<i64> {
  for line in content.lines() {
    let line = line.trim();
    if line.is_empty() {
      continue;
    }
    let Some((_, v)) = line.split_once("term=") else {
      continue;
    };
    let v = v.trim();
    // 取日期部分 YYYY[-/.]MM[-/.]DD
    let nums: Vec<i64> = v
      .split(|c: char| c == '-' || c == '/' || c == '.' || c == ' ' || c == ':')
      .filter_map(|s| s.parse::<i64>().ok())
      .collect();
    if nums.len() >= 3 {
      return Some(days_from_civil(nums[0], nums[1], nums[2]));
    }
    return None;
  }
  None
}

/// 今天(本地日期)的绝对天数
#[allow(dead_code)]
fn today_days() -> i64 {
  use std::time::{SystemTime, UNIX_EPOCH};
  let secs = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|d| d.as_secs() as i64)
    .unwrap_or(0)
    + 8 * 3600; // UTC+8
  days_from_civil_from_secs(secs)
}

/// 秒数(UTC+8) -> 公历天数(与 detect::format_now 的 civil_from_days 同源算法)
#[allow(dead_code)]
fn days_from_civil_from_secs(secs: i64) -> i64 {
  let days = secs.div_euclid(86400);
  let z = days + 719468;
  let era = if z >= 0 { z } else { z - 146096 } / 146097;
  let doe = (z - era * 146097) as i64;
  let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
  let y = yoe as i64 + era * 400;
  let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
  let mp = (5 * doy + 2) / 153;
  let d = doy - (153 * mp + 2) / 5 + 1;
  let m = if mp < 10 { mp + 3 } else { mp - 9 };
  let y = if m <= 2 { y + 1 } else { y };
  days_from_civil(y, m, d)
}

/// 公历(年,月,日)转绝对天数(Howard Hinnant 逆算法)
#[allow(dead_code)]
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
  let y = if m <= 2 { y - 1 } else { y };
  let era = if y >= 0 { y } else { y - 399 } / 400;
  let yoe = (y - era * 400) as i64;
  let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
  let doy = (153 * mp + 2) / 5 + d - 1;
  let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
  era * 146097 + doe - 719468
}

/// 统信 UOS V20: uos-activator-cmd -q
#[cfg(target_os = "linux")]
fn check_uos(items: &mut Vec<CheckItem>) -> (Activation, Vec<CheckItem>, String, Option<String>) {
  let q = run_check("UOS 激活状态查询", "uos-activator-cmd", &["-q"], None);
  let (q_success, q_out) = (q.success, q.output.clone());
  items.push(q);
  let act = if q_success {
    judge(
      &q_out,
      // 官方输出:激活状态=免费授权/已激活/试用期;到期时间=终身有效/日期
      // 精确词,避免宽泛子串误判(如 inactive 含 active)
      &["已激活", "activated", "激活成功", "已授权", "免费授权", "正式授权"],
      &["未激活", "not activated", "未授权", "试用期", "试用", "trial", "expired", "已过期", "inactive", "未激活"],
    )
  } else {
    Activation::Unknown
  };
  let summary = match act {
    Activation::Activated => "统信 UOS 已激活".to_string(),
    Activation::NotActivated => "统信 UOS 未激活".to_string(),
    Activation::Unknown => "统信 UOS 激活状态无法判定,请查看下方明细".to_string(),
    Activation::NotApplicable => "无需激活".to_string(),
  };
  // 授权到期时间(官方输出:到期时间:2025-12-14 / 终身有效)
  let expire = extract_uos_expire(&q_out);
  (act, items.clone(), summary, expire)
}


/// 提取 .kyinfo 的 term 到期时间原文(如 2025-07-29)
#[allow(dead_code)]
fn extract_kyinfo_term(content: &str) -> Option<String> {
  for line in content.lines() {
    let line = line.trim();
    let Some((_, v)) = line.split_once("term=") else {
      continue;
    };
    let v = v.trim().to_string();
    if !v.is_empty() {
      return Some(v);
    }
  }
  None
}

/// 提取统信 uos-activator-cmd -q 输出的到期时间(到期时间:2025-12-14 / 终身有效)
#[allow(dead_code)]
fn extract_uos_expire(output: &str) -> Option<String> {
  for line in output.lines() {
    let line = line.trim();
    if let Some((_, v)) = line.split_once("到期时间") {
      // 到期时间:2026-11-03(半角)或到期时间：2026-11-03(全角),两种冒号都去掉
      let v = v.trim_start_matches(':').trim_start_matches('：').trim().to_string();
      if !v.is_empty() {
        return Some(v);
      }
    }
  }
  None
}

/// 提取 Windows slmgr -xpr 的批量激活到期日期(批量激活将于 2027/2/4 17:50:22 过期 / 永久激活)
#[allow(dead_code)]
fn extract_date(output: &str) -> Option<String> {
  let s = output.to_lowercase();
  if s.contains("永久激活") || s.contains("permanently activated") {
    return Some("永久激活".to_string());
  }
  // 匹配 YYYY/M/D 或 YYYY/M/D HH:MM:SS
  for line in output.lines() {
    let line = line.trim();
    let mut chars = line.chars().peekable();
    let mut digits = String::new();
    while let Some(&c) = chars.peek() {
      if c.is_ascii_digit() || c == '/' || c == ':' || c == ' ' || c == '-' {
        digits.push(c);
        chars.next();
      } else if digits.contains('/') {
        // 可能日期:至少两段数字
        let parts: Vec<&str> = digits.trim().split_whitespace().collect();
        if parts.len() >= 1 && parts[0].contains('/') {
          let seg: Vec<&str> = parts[0].split('/').collect();
          if seg.len() == 3 {
            return Some(parts[0].to_string());
          }
        }
        digits.clear();
        chars.next();
      } else {
        digits.clear();
        chars.next();
      }
    }
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 主上真机 .kyinfo(麒麟 V10 SP1,授权已过期 2025-07-29)
  const KYINFO_EXPIRED: &str = "[dist]name=Kylin-Desktop
milestone=V10
arch=x86_64
beta=False
time=2024-04-07 14:45:43
dist_id=Kylin-Desktop-V10-SP1-2403-Release-20240430-x86_64-2024-04-07 14:45:43
[servicekey]key=0467027
[os]to=term=2025-07-29";

  /// 有效期内授权(term=2027-12-31)
  const KYINFO_VALID: &str = "[dist]name=Kylin-Desktop
milestone=V10
arch=x86_64
[servicekey]key=ABCDEFG
[os]to=term=2027-12-31";

  #[test]
  fn parse_expire_days() {
    let d = kyinfo_expire_days(KYINFO_EXPIRED).expect("parse expired");
    assert_eq!(days_from_civil(2025, 7, 29), d);
    let d2 = kyinfo_expire_days(KYINFO_VALID).expect("parse valid");
    assert_eq!(days_from_civil(2027, 12, 31), d2);
  }

  #[test]
  fn expire_compare_today() {
    let today = today_days();
    assert!(kyinfo_expire_days(KYINFO_EXPIRED).unwrap() < today);
    assert!(kyinfo_expire_days(KYINFO_VALID).unwrap() > today);
  }

  #[test]
  fn days_roundtrip() {
    assert_eq!(days_from_civil(2026, 1, 1), days_from_civil_from_secs(1767225600 + 8 * 3600));
  }
}