# OS Active 系统激活状态检测

<p>
  <img alt="GUI" src="https://img.shields.io/badge/GUI-egui%200.36-orange.svg" />
  <img alt="Platform" src="https://img.shields.io/badge/Kylin%20%7C%20UOS%20%7C%20Ubuntu%20%7C%20Windows-lightgrey.svg" />
  <img alt="Log" src="https://img.shields.io/badge/Log-e--log-blue.svg" />
</p>

<p>
  <a href="https://gitee.com/eternalnight996/os-active">🌟 Gitee</a> |
  <a href="https://github.com/EternalNight996/os-active">🐙 GitHub</a> |
  <a href="https://gitee.com/eternalnight996/os-active/releases">📦 下载</a>
</p>

**一句话**: 跨平台 GUI 工具,一键检测 **银河麒麟 / 统信 UOS / Ubuntu 18.04+ / Windows 10/11** 的系统激活状态,读取设备 **SN**,e-log 把明细写入**程序目录 `logs/<SN>-os-active.log`**,退出输出 ETest 标准 `R<{json}>R`。

## 功能

- ✅ **激活状态检测**: 启动即自动检测,大字英雄区显示 已激活 / 未激活 / 无需激活 / 无法判定
- ✅ **授权到期时间**: GUI 系统信息区显示授权到期(麒麟 `.kyinfo` term / 统信 `到期时间` / Windows 批量激活过期)
- ✅ **SN 读取**: 多方案自动探测(见下文),工具路径可在配置表自配
- ✅ **GUI 明细**: 系统信息(系统/版本/架构/SN/发行版) + 检测明细表格(检测项/命令/判定/原始输出)
- ✅ **e-log 日志**: 明细写入 `logs/<SN>-os-active.log`(SN+项目名,程序目录,避免写 U 盘)
- ✅ **确认流程**: 已激活点「确认激活状态」→ 倒计时自动关闭;未激活时 auto_close 开→自动关窗,关→显示「重新检测」
- ✅ **ETest R 标准输出**: 退出时输出 `R<{json}>R`,仅「激活校验通过 + 已确认」才 `status:true`
- ✅ **配置表自动生成**: 首次运行无 `os-active.toml` 自动生成默认配置(含注释)
- ✅ **tools 工具 + start.sh**: 按 `tools/<工具名>/<架构>/<os>/<工具>` 放置,`start.sh` 一键提权启动+EGL 回退
- ✅ **文件夹部署包**: `just pack` 产出与产测工具一致的部署目录

## 界面预览

| Windows 10 | 银河麒麟 V10 | 统信 UOS V20 |
|---|---|---|
| ![Windows](assets/screen/windows.png) | ![银河麒麟](assets/screen/银河麒麟V10.png) | ![统信 UOS](assets/screen/统信v20.png) |

> 主界面:大字状态英雄区(已激活/未激活) + 底部彩色确认按钮(颜色随状态:绿=已激活/红=未激活/蓝=无需激活/灰=检测中) + 系统信息(含 SN/授权到期) + 检测明细表格 + 日志区;确认后倒计时自动关闭,退出输出 ETest `R<...>R` 结果。

## 激活检测原理

| 系统 | 检测方式 | 判定依据 |
|---|---|---|
| Windows 10/11 | `slmgr.vbs -xpr` + `-dlv` | 永久激活/已激活/将在…到期 → 已激活;通知模式/未授权 → 未激活 |
| 银河麒麟 | `licence -l` + **激活码文件 `/etc/.kyactivation`** + `kylin_activation_check` + `kylin-verify` + `.kyinfo` term | **激活码文件存在=已激活**(权威);term 过期=未激活;term 未来≠已激活(仅预置期限) |
| 统信 UOS V20 | `uos-activator-cmd -q`(官方) | 激活状态=免费授权/已激活 → 已激活;试用期/未激活 → 未激活 |
| Ubuntu 18.04+ | 无激活机制 | 无需激活 |

> 麒麟激活判定链:已激活必须有证据(激活码文件/官方命令/licence),`.kyinfo` 的 term 仅用于判过期与显示到期时间。

## SN 读取

多方案自动探测,工具路径可在 `os-active.toml` 的 `[sn].tool_path` 自配:

| # | 方案 | 来源 | 需 root |
|---|---|---|---|
| 1 | ByoDmi 工具 `-smbiosinfo` | DMI SMBIOS Type1 Serial(对齐产测烧录位置) | 是 |
| 2 | `/sys/class/dmi/id/product_serial` | DMI sysfs | 否(优先) |
| 3 | `/sys/class/dmi/id/board_serial` | DMI 主板序列号 | 否 |
| 4 | `dmidecode -s system-serial-number` | DMI 标准命令 | 是 |
| 5 | `/etc/machine-id` | 设备唯一 ID(兜底) | 否 |
| 6 | `Win32_BIOS.SerialNumber` | Windows BIOS | 否 |

> 读取链 `ByoDmi → product_serial → board_serial → dmidecode → machine-id` 逐级回退,过滤占位符(`To be filled`/`None`),日志输出各来源校验明细。

## 使用

### Linux(麒麟/统信/Ubuntu)

```bash
./start.sh                # 一键:root 提权(读 SN)+ 图形环境保留 + EGL 三级回退 + 启动
# 或手动(需 root 读 SN,EGL 回退):
E_AUTOTEST_GL=egl ./os-active
```

### Windows

```bash
os-active.exe
```

## 配置表 os-active.toml

首次运行自动生成(含注释),可修改后重启生效:

```toml
[app]
close_after_secs = 3    # PASS 后倒计时 N 秒自动关闭窗口(默认 3)
auto_close = false      # true:激活成功自动确认并倒计时关闭;false:未激活时显示「重新检测」

[sn]
# SN 工具(ByoDmi)路径,可自配;留空自动探测 tools/ByoDmi/<架构>/<os>/ByoDmi
# tool_path = "tools/ByoDmi/x86_64/linux/ByoDmi"
require_sn = true      # 是否校验 SN 不为空(空则日志告警)
```

## ETest R 输出

退出时(on_exit)输出一行 `R<{json}>R`(ETest 插件标准,主程序正则提取判定):

```json
R<{"content":"PASS;银河麒麟已激活;auto确认","status":true,"opts":{"activation":"已激活","sn":"MT81...","expire_at":"2027-05-29",...}}>R
```

- `status:true` 需**同时满足**: 1) 激活校验通过 2) 已确认(按钮 或 auto_close=true)
- `content`: `PASS;...`(通过) / `NG;...`(未通过,含原因)
- `opts`: 系统信息 + SN + 授权到期 + 激活判定 + 确认方式 + 检测明细

## 日志

- **程序目录 `logs/<SN>-os-active.log`**(SN+项目名;当前软件路径,避免写 U 盘;不可写则 fallback 家目录): 检测明细(每条命令 + 原始输出 + 判定)
- `logs/bug.os-active.log`: panic 崩溃现场
- GUI 底部展示日志文件路径与最新日志,一键打开日志目录

## 构建与打包

依赖 **Rust**(≥1.85)与 **just**:

```bash
cargo install just
just setup                 # 一键装齐工具链
just doctor                # 环境自检
just run                   # 本机运行
just dist                  # Windows 包 → dist/os-active-v<版本>.zip
just deb-native            # Linux deb(Linux 原生构建,推荐)
just pack                  # 文件夹部署包 → dist/os-active-pack-v<版本>/(主程序+tools+README+start.sh+config)
```

Linux 二进制以 glibc 2.27 为基线,一次构建可在 Ubuntu 18.04+ / Kylin / UOS 全系运行;`start.sh`/deb 启动器内置 GLX→EGL→软件渲染三级显示回退。

## tools 工具目录

`tools/` 存放产测/硬件工具,按 **工具名 → 架构 → 操作系统 → 具体工具** 分层。**具体工具不上传 git**(体积大/产线定制),用户自行放置,程序自动探测:

```
tools/
  <工具名>/
    <架构>/          # x86_64 / aarch64 ...
      <操作系统>/     # linux / windows ...
        具体工具
```

当前预留:ByoDmi(DMI/SN 烧录与读取)。详见 `tools/README.md`。

## 发布与双源管理(GitHub + Gitee)

推 tag 自动完成:**Windows zip + Linux deb 跨平台构建** → 发布 **GitHub Release** → 同步分支/tag 到 Gitee → 上传 **Gitee 发行版**(`.github/workflows/release.yml`)。首次需在 GitHub Actions secrets 配 `GITEE_TOKEN`。

```bash
git tag v0.1.0 && git push origin master github v0.1.0   # 双源推 tag 触发 CI
```

## 目录结构

```
src/            # 源码(main/app/config/data/detect)
tools/          # 工具(不上传 git,用户自添;按 工具名/架构/os/工具 分层)
assets/         # 界面截图
docs/           # 文档(含 TP100 分析)
vendor/         # EGL patch 的 eframe 0.36.1
start.sh        # Linux 启动脚本(提权+EGL 回退+输入法)
justfile        # 构建/打包统一入口
.github/        # CI 发布工作流
scripts/        # Gitee 发布/双远程脚本
```

## 第三方修正

- **eframe 0.36.1(vendor)**—— 国产 OS EGL 回退 patch(`E_AUTOTEST_GL=egl` → `PreferEgl`,绕开 UOS/Kylin 损坏的 GLX;Linux 默认 EGL)