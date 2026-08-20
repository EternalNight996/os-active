//! 系统识别:Linux 解析 /etc/os-release;Windows 查询注册表 ProductName
#[cfg(target_os = "linux")]
use e_log::preload::*;
use super::model::OsInfo;
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
  Some(s)
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
pub fn get_sn() -> Option<String> {
  #[cfg(windows)]
  {
    // 用 std::process::Command 直接执行(e-utils Cmd 对 "powershell" 有 ExeType 特殊处理,会把 exe 当 -Command 执行)
    use std::os::windows::process::CommandExt;
    if let Ok(out) = std::process::Command::new("powershell")
      .args(["-NoProfile", "-Command", "(Get-CimInstance Win32_BIOS).SerialNumber"])
      .creation_flags(0x08000000) // CREATE_NO_WINDOW:release 禁止弹 cmd 窗口
      .output()
    {
      let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
      if !s.is_empty() && !s.eq_ignore_ascii_case("to be filled by o.e.m.") {
        return Some(s);
      }
    }
    None
  }
  #[cfg(target_os = "linux")]
  {
    // 多来源读取(优先级):ByoDmi -> product_serial -> board_serial -> dmidecode -> machine-id
    // 校验:过滤占位符,记录各来源值
    let byodmi = byodmi_sn();
    let prod = read_dmi_serial("/sys/class/dmi/id/product_serial");
    let board = read_dmi_serial("/sys/class/dmi/id/board_serial");
    // 校验明细(日志)
    info!(
      "SN 校验: ByoDmi={:?} product_serial={:?} board_serial={:?}",
      byodmi, prod, board
    );
    // 优先 ByoDmi(对齐 TP100 烧录位置),其次 product_serial,再 board_serial
    if let Some(s) = byodmi {
      return Some(s);
    }
    if let Some(s) = prod {
      return Some(s);
    }
    if let Some(s) = board {
      return Some(s);
    }
    // dmidecode(需 root,兜底)
    if let Ok(o) = Cmd::new("dmidecode").args(["-s", "system-serial-number"]).output() {
      let s = o.stdout.trim().to_string();
      if !s.is_empty() {
        return Some(s);
      }
    }
    // machine-id(机器唯一 ID 兜底)
    if let Ok(s) = std::fs::read_to_string("/etc/machine-id") {
      let s = s.trim().to_string();
      if !s.is_empty() {
        return Some(s);
      }
    }
    None
  }
  #[cfg(not(any(target_os = "linux", windows)))]
  {
    None
  }
}