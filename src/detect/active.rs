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
pub fn check(os: &OsInfo) -> (Activation, Vec<CheckItem>, String) {
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
fn check_windows() -> (Activation, Vec<CheckItem>, String) {
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
  (act, items, summary)
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
fn check_linux(os: &OsInfo) -> (Activation, Vec<CheckItem>, String) {
  let mut items: Vec<CheckItem> = vec![];
  match os.distro_id.as_str() {
    "ubuntu" => {
      items.push(CheckItem::new("激活机制", "n/a", true, "Ubuntu 无系统激活机制,安装即完整授权".into(), "无需激活"));
      (Activation::NotApplicable, items, "Ubuntu 无需激活".to_string())
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
      (Activation::Unknown, items, "无法识别的系统发行版".to_string())
    }
  }
}

/// 银河麒麟 V10: licence -l + /etc/.kyinfo
#[cfg(target_os = "linux")]
fn check_kylin(items: &mut Vec<CheckItem>) -> (Activation, Vec<CheckItem>, String) {
  let lic = run_check("麒麟授权查询", "licence", &["-l"], None);
  let kyinfo = read_file_check("/etc/.kyinfo", "麒麟授权文件(.kyinfo)");
  let (lic_success, lic_out) = (lic.success, lic.output.clone());
  let kyinfo_success = kyinfo.success;
  items.push(lic);
  items.push(kyinfo);

  let act = if lic_success {
    judge(
      &lic_out,
      &["已激活", "activated", "永久授权", "授权成功", "valid", "已授权"],
      &["未激活", "not activated", "试用", "trial", "invalid", "expired", "未授权", "未激活"],
    )
  } else if kyinfo_success {
    // licence 工具不可用但授权文件存在:弱判定已激活
    Activation::Activated
  } else {
    Activation::Unknown
  };
  let summary = match act {
    Activation::Activated => "银河麒麟已激活".to_string(),
    Activation::NotActivated => "银河麒麟未激活".to_string(),
    Activation::Unknown => "银河麒麟激活状态无法判定,请查看下方明细".to_string(),
    Activation::NotApplicable => "无需激活".to_string(),
  };
  (act, items.clone(), summary)
}

/// 统信 UOS V20: uos-activator-cmd -q
#[cfg(target_os = "linux")]
fn check_uos(items: &mut Vec<CheckItem>) -> (Activation, Vec<CheckItem>, String) {
  let q = run_check("UOS 激活状态查询", "uos-activator-cmd", &["-q"], None);
  let (q_success, q_out) = (q.success, q.output.clone());
  items.push(q);
  let act = if q_success {
    judge(
      &q_out,
      &["已激活", "activated", "激活成功", "已授权"],
      &["未激活", "not activated", "未授权", "试用", "trial", "expired"],
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
  (act, items.clone(), summary)
}