//! 系统识别:Linux 解析 /etc/os-release;Windows 查询注册表 ProductName
#[cfg(target_os = "linux")]
use e_log::preload::*;
use super::model::{CheckItem, OsInfo};
#[cfg(any(windows, target_os = "linux"))]
use e_utils::cmd::Cmd;

/// 识别当前操作系统
pub fn detect_os() -> OsInfo {
  #[cfg(target_os = "linux")]
  {
    detect_linux()
  }
  #[cfg(windows)]
  {
    detect_windows()
  }
  #[cfg(not(any(target_os = "linux", windows)))]
  {
    OsInfo {
      name: std::env::consts::OS.to_string(),
      version: String::new(),
      arch: std::env::consts::ARCH.to_string(),
      distro_id: std::env::consts::OS.to_string(),
      pretty: String::new(),
      sn: String::new(),
      cpu: String::new(),
      sn_source: String::new(),
    }
  }
}

/// 解析 /etc/os-release 的 key="value"
#[cfg(target_os = "linux")]
fn parse_os_release() -> (String, String, String) {
  let content = match std::fs::read_to_string("/etc/os-release") {
    Ok(c) => c,
    Err(_) => return (String::new(), String::new(), String::new()),
  };
  let mut id = String::new();
  let mut pretty = String::new();
  let mut version = String::new();
  for line in content.lines() {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
      continue;
    }
    if let Some((k, v)) = line.split_once('=') {
      let v = v.trim().trim_matches('"').to_string();
      match k.trim() {
        "ID" => id = v,
        "PRETTY_NAME" => pretty = v.clone(),
        "VERSION_ID" => version = v,
        "VERSION" if version.is_empty() => version = v,
        _ => {}
      }
    }
  }
  (id, pretty, version)
}

#[cfg(target_os = "linux")]
fn detect_linux() -> OsInfo {
  let (id, pretty, version) = parse_os_release();
  let arch = std::env::consts::ARCH.to_string();
  // 麒麟 V10 授权文件(激活过的系统会在 /etc 下生成 .kyinfo)
  let has_kyinfo = std::path::Path::new("/etc/.kyinfo").exists();
  let name = match id.as_str() {
    "kylin" => "银河麒麟".to_string(),
    "uos" | "deepin" => "统信 UOS".to_string(),
    "ubuntu" => "Ubuntu".to_string(),
    _ => {
      if has_kyinfo {
        "银河麒麟".to_string()
      } else if id.is_empty() {
        "Linux".to_string()
      } else {
        id.clone()
      }
    }
  };
  OsInfo {
    name,
    version,
    arch,
    distro_id: id,
    pretty,
    sn: get_sn().unwrap_or_default(),
    cpu: get_cpu(),
    sn_source: String::new(),
  }
}

#[cfg(windows)]
fn detect_windows() -> OsInfo {
  // 读取注册表 ProductName:HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion
  let product = Cmd::new("reg")
    .args([
      "query",
      "HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion",
      "/v",
      "ProductName",
    ])
    .output()
    .map(|o| o.stdout)
    .unwrap_or_default();
  // 提取 REG_SZ 后面的值
  let version = product
    .lines()
    .find_map(|l| {
      let l = l.trim();
      l.split_once("REG_SZ").map(|(_, v)| v.trim().to_string())
    })
    .unwrap_or_else(|| "10+".to_string());
  let pretty = format!("Windows {}", version);
  OsInfo {
    name: "Windows".to_string(),
    version,
    arch: std::env::consts::ARCH.to_string(),
    distro_id: "windows".to_string(),
    pretty,
    sn: get_sn().unwrap_or_default(),
    cpu: get_cpu(),
  }
}



/// 用 plugins/<arch>/ByoDmi 读取 DMI SN(解析 -smbiosinfo 的 Serial Number,需 root)
#[cfg(target_os = "linux")]
fn byodmi_sn() -> Option<String> {
  // 路径:配置 [sn].tool_path 优先,否则默认探测 tools/ByoDmi/<架构>/<os>/ByoDmi
  let byodmi = match sn_tool_path() {
    Some(p) => std::path::PathBuf::from(p),
    None => {
      let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
      exe_dir
        .join("tools")
        .join("ByoDmi")
        .join(std::env::consts::ARCH)
        .join(std::env::consts::OS)
        .join("ByoDmi")
    }
  };
  if !byodmi.exists() {
    return None;
  }
  let out = match std::process::Command::new(&byodmi).arg("-smbiosinfo").output() {
    Ok(o) => o,
    Err(e) => {
      info!("[SN] ByoDmi 执行失败(可能未安装或平台不支持): {e}");
      return None;
    }
  };
  if !out.status.success() {
    // 段错误/异常退出(信号 11 等):记录并回退其他 SN 来源
    info!("[SN] ByoDmi 执行异常(退出码 {:?}),回退其他 SN 来源", out.status.code());
    return None;
  }
  let text = String::from_utf8_lossy(&out.stdout);
  // 精确匹配 Type1 Serial Number 字段(排除 UUID/handle 等其他 hex 字段)
  for line in text.lines() {
    let l = line.trim();
    let lower = l.to_lowercase();
    if lower.contains("serial number") && !lower.contains("uuid") {
      let v = if let Some((_, v)) = l.split_once(':') {
        v.trim().to_string()
      } else if let Some((_, v)) = l.split_once('=') {
        v.trim().to_string()
      } else {
        continue;
      };
      if let Some(s) = sanitize_sn(&v) {
        return Some(s);
      }
    }
  }
  None
}


/// 从配置文件 os-active.toml 读取 [sn].tool_path(自配 SN 工具路径)
#[cfg(target_os = "linux")]
fn sn_tool_path() -> Option<String> {
  let cfg = std::env::current_dir().ok()?.join("os-active.toml");
  let content = std::fs::read_to_string(cfg).ok()?;
  for line in content.lines() {
    let l = line.trim();
    if let Some((_, v)) = l.split_once("tool_path") {
      let v = v.trim().trim_start_matches('=').trim().trim_matches('"').trim();
      if !v.is_empty() {
        return Some(v.to_string());
      }
    }
  }
  None
}


/// SN 值校验:过滤占位符/过短(<8 字符)/空值,返回有效 SN
#[cfg(target_os = "linux")]
fn sanitize_sn(v: &str) -> Option<String> {
  let s = v.trim().to_string();
  if s.is_empty() {
    return None;
  }
  let lower = s.to_lowercase();
  if lower.contains("none") || lower.contains("to be filled") || lower.contains("default string") || lower.contains("null") {
    return None;
  }
  if s.chars().count() < 8 {
    return None; // 过短,疑似无效/被截断
  }
  // 规范化:去除内部空格(VMware UUID 格式带空格,显示更干净)
  Some(s.replace(' ', ""))
}

/// 读 DMI 序列号文件并过滤无效值
#[cfg(target_os = "linux")]
fn read_dmi_serial(path: &str) -> Option<String> {
  if let Ok(s) = std::fs::read_to_string(path) {
    return sanitize_sn(&s);
  }
  None
}

/// 获取设备序列号(SN):Windows BIOS SerialNumber;Linux DMI product_serial(优先)/dmidecode
/// 获取不到返回 None(空 SN 时日志用默认名)
/// 探测 SN,返回 (SN, 各来源明细, 采用来源)
pub fn probe_sn() -> (Option<String>, Vec<CheckItem>, String) {
  let mut probes: Vec<CheckItem> = vec![];
  let mut sn: Option<String> = None;
  let mut source = "未获取".to_string();

  #[cfg(target_os = "linux")]
  {
    // 1) ByoDmi
    let byodmi = byodmi_sn();
    match &byodmi {
      Some(s) => probes.push(CheckItem::new("SN-ByoDmi", "ByoDmi -smbiosinfo", true, s.clone(), "成功")),
      None => probes.push(CheckItem::new("SN-ByoDmi", "ByoDmi -smbiosinfo", false, "不可用(未放置工具或平台不支持)".to_string(), "跳过")),
    }
    // 2) product_serial
    let prod = read_dmi_serial("/sys/class/dmi/id/product_serial");
    match &prod {
      Some(s) => probes.push(CheckItem::new("SN-DMI序列号", "cat /sys/class/dmi/id/product_serial", true, s.clone(), "成功")),
      None => probes.push(CheckItem::new("SN-DMI序列号", "cat /sys/class/dmi/id/product_serial", false, "读取失败(需 root 权限或虚拟机无 SN)".to_string(), "跳过")),
    }
    // 3) board_serial
    let board = read_dmi_serial("/sys/class/dmi/id/board_serial");
    match &board {
      Some(s) => probes.push(CheckItem::new("SN-主板序列号", "cat /sys/class/dmi/id/board_serial", true, s.clone(), "成功")),
      None => probes.push(CheckItem::new("SN-主板序列号", "cat /sys/class/dmi/id/board_serial", false, "读取失败(需 root 权限或虚拟机无 SN)".to_string(), "跳过")),
    }
    // 4) dmidecode
    let dmidec = Cmd::new("dmidecode")
      .args(["-s", "system-serial-number"])
      .output()
      .ok()
      .and_then(|o| {
        let s = o.stdout.trim().to_string();
        sanitize_sn(&s)
      });
    match &dmidec {
      Some(s) => probes.push(CheckItem::new("SN-dmidecode", "dmidecode -s system-serial-number", true, s.clone(), "成功")),
      None => probes.push(CheckItem::new("SN-dmidecode", "dmidecode -s system-serial-number", false, "读取失败(需 root 权限)".to_string(), "跳过")),
    }
    // 5) machine-id
    let mid = std::fs::read_to_string("/etc/machine-id")
      .ok()
      .map(|s| s.trim().to_string());
    match &mid {
      Some(s) => probes.push(CheckItem::new("SN-machine-id", "cat /etc/machine-id", true, s.clone(), "兜底(非厂商SN)")),
      None => probes.push(CheckItem::new("SN-machine-id", "cat /etc/machine-id", false, "读取失败".to_string(), "跳过")),
    }
    // 按优先级取第一个有效
    for (p, name) in [
      (&byodmi, "ByoDmi"),
      (&prod, "DMI product_serial"),
      (&board, "DMI board_serial"),
      (&dmidec, "dmidecode"),
    ] {
      if sn.is_none() {
        if let Some(s) = p {
          sn = Some(s.clone());
          source = name.to_string();
        }
      }
    }
    info!("SN 校验: ByoDmi={byodmi:?} product_serial={prod:?} board_serial={board:?} dmidecode={dmidec:?} -> 采用:{source}");
  }

  #[cfg(windows)]
  {
    use std::os::windows::process::CommandExt;
    let bios = std::process::Command::new("powershell")
      .args(["-NoProfile", "-Command", "(Get-CimInstance Win32_BIOS).SerialNumber"])
      .creation_flags(0x08000000)
      .output()
      .ok()
      .and_then(|o| {
        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if !s.is_empty() && !s.eq_ignore_ascii_case("to be filled by o.e.m.") {
          Some(s)
        } else {
          None
        }
      });
    match &bios {
      Some(s) => probes.push(CheckItem::new("SN-Win32_BIOS", "Get-CimInstance Win32_BIOS.SerialNumber", true, s.clone(), "成功")),
      None => probes.push(CheckItem::new("SN-Win32_BIOS", "Get-CimInstance Win32_BIOS.SerialNumber", false, "读取失败".to_string(), "跳过")),
    }
    if let Some(s) = bios {
      sn = Some(s);
      source = "Win32_BIOS".to_string();
    }
    info!("SN 校验: Win32_BIOS={sn:?} -> 采用:{source}");
  }

  (sn, probes, source)
}

/// 获取设备序列号(SN),获取不到返回 None(空 SN 时日志用默认名)
pub fn get_sn() -> Option<String> {
  probe_sn().0
}
/// 识别 CPU 型号(国产:海光 Hygon / 兆芯 Zhaoxin;Intel/AMD)
pub fn get_cpu() -> String {
  #[cfg(target_os = "linux")]
  {
    if let Ok(s) = std::fs::read_to_string("/proc/cpuinfo") {
      let mut vendor = String::new();
      for line in s.lines() {
        let l = line.trim();
        if let Some((k, v)) = l.split_once(':') {
          let k = k.trim();
          let v = v.trim();
          if k == "model name" && !vendor.is_empty() {
            return format!("{v} ({vendor})");
          }
          if k == "vendor_id" {
            vendor = if v.contains("Hygon") {
              "海光 Hygon".to_string()
            } else if v.contains("Centaur") {
              "兆芯 Zhaoxin".to_string()
            } else if v.contains("GenuineIntel") {
              "Intel".to_string()
            } else if v.contains("AuthenticAMD") {
              "AMD".to_string()
            } else {
              v.to_string()
            };
          }
        }
      }
      if !vendor.is_empty() {
        return vendor;
      }
    }
    String::new()
  }
  #[cfg(windows)]
  {
    use std::os::windows::process::CommandExt;
    if let Ok(out) = std::process::Command::new("powershell")
      .args(["-NoProfile", "-Command", "(Get-CimInstance Win32_Processor).Name"])
      .creation_flags(0x08000000)
      .output()
    {
      let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
      if !s.is_empty() {
        return s;
      }
    }
    String::new()
  }
  #[cfg(not(any(target_os = "linux", windows)))]
  {
    String::new()
  }
}