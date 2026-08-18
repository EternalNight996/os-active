//! 系统识别:Linux 解析 /etc/os-release;Windows 查询注册表 ProductName
use super::model::OsInfo;
#[cfg(windows)]
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
  }
}