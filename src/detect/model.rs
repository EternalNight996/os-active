//! 检测结果数据结构
use serde::{Deserialize, Serialize};

/// 系统信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OsInfo {
  /// 系统名(中文展示,如 银河麒麟 / 统信 UOS / Ubuntu / Windows)
  pub name: String,
  /// 版本(如 V10 SP1 / V20 / 20.04 / 11)
  pub version: String,
  /// CPU 架构(x86_64 / aarch64 ...)
  pub arch: String,
  /// 发行版 ID(kylin / uos / ubuntu / windows)
  pub distro_id: String,
  /// os-release PRETTY_NAME 原文(仅 Linux)
  pub pretty: String,
  /// 设备序列号(SN,获取不到为空)
  pub sn: String,
  /// CPU(海光 Hygon / 兆芯 Zhaoxin / Intel / AMD ...)
  pub cpu: String,
}

/// 激活状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Activation {
  /// 已激活
  Activated,
  /// 未激活
  NotActivated,
  /// 无需激活(如 Ubuntu)
  NotApplicable,
  /// 无法判定
  Unknown,
}
impl Activation {
  /// 中文状态文案
  pub fn label(&self) -> &'static str {
    match self {
      Self::Activated => "已激活",
      Self::NotActivated => "未激活",
      Self::NotApplicable => "无需激活",
      Self::Unknown => "无法判定",
    }
  }
}

/// 单条检测明细
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckItem {
  /// 检测项名(如 激活状态查询 / 详细授权信息)
  pub name: String,
  /// 实际执行的命令行(可复现)
  pub command: String,
  /// 命令是否成功执行(exit ok)
  pub success: bool,
  /// 原始输出(日志体现明细的核心)
  pub output: String,
  /// 判定说明
  pub verdict: String,
}
impl CheckItem {
  pub fn new(name: &str, command: &str, success: bool, output: String, verdict: &str) -> Self {
    Self {
      name: name.to_string(),
      command: command.to_string(),
      success,
      output,
      verdict: verdict.to_string(),
    }
  }
}

/// 一次完整检测的结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectResult {
  pub os: OsInfo,
  pub activation: Activation,
  /// 一句话判定摘要
  pub summary: String,
  pub items: Vec<CheckItem>,
  /// 授权到期时间(麒麟 .kyinfo term / 统信 到期时间 / Windows 批量激活过期,解析不出为 None)
  pub expire_at: Option<String>,
  /// 检测时间(本地, %Y-%m-%d %H:%M:%S)
  pub checked_at: String,
}