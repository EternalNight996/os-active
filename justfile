# ============================================================
# os-active 构建 / 打包 / 验证命令 (just)
#
# 复刻自 etest(自动化测试平台) 的 justfile,仅保留本工程所需 recipe。
#
# 【新电脑一键部署】只需 3 步:
#   1. cargo install just        (Rust 未装先装: https://rustup.rs)
#   2. just setup                # 全自动装齐工具链(winget 装 rustup/VS/zig + cargo 工具)
#   3. 新开终端 → just dist      # 一键构建 Windows+Linux 并打包 → dist/os-active-v<版本>.zip
#   或: just run 本机运行 / just deb 打 Linux deb 包
#
# 说明:
#   - Windows 构建必须 MSVC(vswhere 自动定位 VS,GNU dlltool 有缺陷不可用);
#     统一走 PowerShell(Windows 自带),不依赖 Git Bash/cmd 脚本。
# ============================================================
set windows-shell := ["powershell", "-NoProfile", "-Command"]

# ---- 统一变量区(改这里即可,勿散落硬编码)----
bin          := "os-active"                    # crate 名(产物名随它变)
profile      := "release"                      # release / debug
target_linux := "x86_64-unknown-linux-gnu.2.27"  # zigbuild glibc 2.27 基线(Ubuntu18+/Kylin/UOS)
target_short := "x86_64-unknown-linux-gnu"       # zigbuild 产物目录名
dist_dir     := "dist"                           # 打包输出目录
win_exe      := "target/" + profile + "/" + bin + ".exe"
linux_bin    := "target/" + target_short + "/" + profile + "/" + bin

# ---- 开发机 VM 验证环境(参考 etest,按实际调整;不影响本地运行/打包)----
vm_host  := "test@192.168.128.50"
vm_ssh   := "ssh -o BatchMode=yes -o StrictHostKeyChecking=no -i vm/id_vm"
vm_scp   := "scp -o BatchMode=yes -o StrictHostKeyChecking=no -i vm/id_vm"
vm_vmx   := "F:/MyApp/eternal/os-active/vm/os-active.vmx"

default:
    @just --list
    @Write-Host ''
    @Write-Host '一键部署:'
    @Write-Host '  just setup        # 首次:全自动装齐工具链(rustup/VS/zig/MSVC)'
    @Write-Host '  just dist         # 一键构建 Windows+Linux 并打包 -> dist/os-active-v<版本>.zip'
    @Write-Host '常用:'
    @Write-Host '  just run          # 本机运行(Windows MSVC)'
    @Write-Host '  just deb          # 打 Linux deb 包 -> ' + {{dist_dir}} + '/'
    @Write-Host '  just doctor       # 环境自检  |  just test 单元测试  |  just version 版本号'

# ---- 辅助(私有,内部复用)----

# MSVC 前缀:vswhere 定位 VS -> cmd 设置 vcvars + MSVC 工具链 -> 执行传入命令
# (GNU dlltool 有缺陷,Windows 构建/测试必须走 MSVC;just 每行独立进程,须单行执行)
[private]
msvc cmd:
    @$vs = (& 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe' -all -products * -property installationPath 2>$null | Select-Object -Last 1); if (-not $vs) { $vs = (& 'C:\Program Files\Microsoft Visual Studio\Installer\vswhere.exe' -all -products * -property installationPath 2>$null | Select-Object -Last 1) }; if (-not $vs) { Write-Host '未找到 Visual Studio: 需 VS2022 Build Tools(MSVC v143 x64)。安装: https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022'; exit 1 }; cmd /c "call `"$vs\VC\Auxiliary\Build\vcvars64.bat`" >nul && set `"RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc`" && {{cmd}}"

# 交叉编译工具检查(build-linux / check / deb 复用)
[private]
ensure-tools:
    @if (-not (Get-Command zig -ErrorAction SilentlyContinue)) { Write-Host '缺少 zig: 请先执行 just setup 一键安装'; exit 1 }
    @if (-not (Get-Command cargo-zigbuild -ErrorAction SilentlyContinue)) { Write-Host '缺少 cargo-zigbuild: 请先执行 just setup 一键安装'; exit 1 }

# 打印单个构建产物
[private]
artifact path:
    @Get-Item {{path}} | Select-Object FullName, Length

# ---- 环境自检 / 安装 ----

# 环境自检:分级检查[必需]本地运行 / [打包]Linux deb
doctor:
    @Write-Host '=== os-active 环境自检 ==='
    @Write-Host '--- [必需] 本地运行 ---'
    @if (Get-Command just -ErrorAction SilentlyContinue) { Write-Host '[OK]   just' } else { Write-Host '[缺] just -> cargo install just' }
    @if (Get-Command cargo -ErrorAction SilentlyContinue) { Write-Host '[OK]   cargo (Rust)' } else { Write-Host '[缺] Rust -> https://rustup.rs' }
    @if (rustup toolchain list 2>$null | Select-String 'windows-msvc') { Write-Host '[OK]   rustup MSVC toolchain' } else { Write-Host '[缺] MSVC toolchain -> rustup toolchain install stable-x86_64-pc-windows-msvc' }
    @if (& 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe' -all -products * -property installationPath 2>$null | Select-Object -Last 1) { Write-Host '[OK]   Visual Studio (MSVC)' } else { Write-Host '[缺] Visual Studio Build Tools -> https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022' }
    @Write-Host '--- [打包] Linux deb ---'
    @if (Get-Command cargo-zigbuild -ErrorAction SilentlyContinue) { Write-Host '[OK]   cargo-zigbuild' } else { Write-Host '[缺] cargo-zigbuild -> cargo install cargo-zigbuild' }
    @if (Get-Command cargo-deb -ErrorAction SilentlyContinue) { Write-Host '[OK]   cargo-deb' } else { Write-Host '[缺] cargo-deb -> cargo install cargo-deb' }
    @if (Get-Command zig -ErrorAction SilentlyContinue) { Write-Host '[OK]   zig (交叉编译器)' } else { Write-Host '[缺] zig -> just setup 自动安装 或 https://ziglang.org/download/' }

# 一键安装依赖(全自动:rustup/VS Build Tools/zig 走 winget,cargo 工具走 cargo install)
# 装完打开新终端使 PATH 生效;之后 just doctor 复查 → just run / just dist
setup:
    @if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) { winget install --id Rustlang.Rustup --silent --accept-package-agreements --accept-source-agreements } else { Write-Host 'rustup 已安装,跳过' }
    @if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { Write-Host 'cargo 不可用: 请打开新终端(让 winget 安装的 rustup 生效),或手动安装 Rust: https://rustup.rs'; exit 1 }
    @if (-not (rustup toolchain list 2>$null | Select-String 'windows-msvc')) { rustup toolchain install stable-x86_64-pc-windows-msvc } else { Write-Host 'MSVC 工具链已安装,跳过' }
    @if (-not (& 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe' -all -products * -property installationPath 2>$null | Select-Object -Last 1)) { winget install --id Microsoft.VisualStudio.2022.BuildTools --silent --accept-package-agreements --accept-source-agreements --override "--quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended" } else { Write-Host 'VS Build Tools 已安装,跳过' }
    @if (-not (Get-Command zig -ErrorAction SilentlyContinue)) { winget install -e --id zig.zig --silent --accept-source-agreements --accept-package-agreements } else { Write-Host 'zig 已安装,跳过' }
    @if (-not (Get-Command cargo-zigbuild -ErrorAction SilentlyContinue)) { cargo install cargo-zigbuild --locked } else { Write-Host 'cargo-zigbuild 已安装,跳过' }
    @if (-not (Get-Command cargo-deb -ErrorAction SilentlyContinue)) { cargo install cargo-deb } else { Write-Host 'cargo-deb 已安装,跳过' }
    @Write-Host '环境就绪。请打开新终端使 PATH 生效,然后: just doctor 复查 / just run / just dist'

# ---- 本地运行 / Windows 构建 ----

# 本机运行(Windows MSVC;Linux 直接 cargo run)
run:
    @{{ if os() == 'windows' { 'just msvc "cargo run --' + profile + '"' } else { 'cargo run --' + profile } }}

# Windows 发布版编译(MSVC 工具链)
build-win:
    @just msvc "cargo build --{{profile}}"
    @just artifact {{win_exe}}

# 单元测试(Windows 走 MSVC;Linux 直接 cargo test)
test:
    @{{ if os() == 'windows' { 'just msvc "cargo test"' } else { 'cargo test' } }}

# 双目标编译检查(Linux 走 zigbuild)
check:
    @{{ if os() == 'windows' { 'just msvc "cargo check"' } else { 'cargo check' } }}
    @{{ if os() == 'windows' { 'just ensure-tools' } else { 'command -v zig >/dev/null && command -v cargo-zigbuild >/dev/null || { echo "缺少 zig/cargo-zigbuild,请先安装"; exit 1; }' } }}
    @{{ if os() == 'windows' { 'just msvc "cargo zigbuild --target ' + target_linux + '"' } else { 'cargo zigbuild --target ' + target_linux } }}

# ---- Linux 交叉编译 / deb 打包 ----

# Linux 交叉编译(zig cc, glibc 2.27 基线)
build-linux:
    @just ensure-tools
    @just msvc "cargo zigbuild --{{profile}} --target {{target_linux}}"
    @just artifact {{linux_bin}}

# 生成 deb 打包资源(启动器/桌面入口内联,输出到 target/packaging)
# 三级显示后端回退:GLX -> EGL -> 软件渲染(部分国产 OS 的 GLX 是"假成功"会崩溃)
gen-packaging:
    @New-Item -ItemType Directory -Force -Path target/packaging | Out-Null
    @[IO.File]::WriteAllText('target/packaging/os-active-wrapper.sh', (@( \
        '#!/bin/sh', \
        '# os-active 启动器(多级显示后端回退)', \
        '# 部分国产 OS(如统信 UOS 1070)X server 的 GLX 扩展损坏,任何 GLX 程序会 GLXBadContextTag 崩溃;EGL 不走 GLX 可绕开', \
        'BIN=/usr/lib/os-active/os-active.bin', \
        'cd "$HOME" || exit 1', \
        'mkdir -p "$HOME/os-active/logs"', \
        'run() { "$@"; rc=$?; [ "$rc" -eq 0 ] || [ "$rc" -eq 130 ]; }', \
        '# 1) 默认: GLX 优先(正常机器)', \
        'run "$BIN" "$@" && exit 0', \
        'echo "[os-active] GLX 模式失败(exit $rc),尝试 EGL 后端..."', \
        '# 2) EGL 硬件加速(绕开损坏的 GLX)', \
        'run env E_AUTOTEST_GL=egl "$BIN" "$@" && exit 0', \
        'echo "[os-active] EGL 模式失败(exit $rc),尝试 EGL + 软件渲染..."', \
        '# 3) EGL + llvmpipe 软件渲染(兜底)', \
        'run env E_AUTOTEST_GL=egl LIBGL_ALWAYS_SOFTWARE=1 "$BIN" "$@" && exit 0', \
        'echo "[os-active] 所有显示后端均失败,请反馈日志。"', \
        'exit 1') -join "`n"))
    @[IO.File]::WriteAllText('target/packaging/os-active.desktop', (@( \
        '[Desktop Entry]', \
        'Type=Application', \
        'Name=OS Active', \
        'Name[zh_CN]=系统激活状态检测', \
        'Comment=Check OS activation status', \
        'Comment[zh_CN]=检测系统激活状态', \
        'Exec=os-active', \
        'Terminal=false', \
        'Categories=System;Utility;') -join "`n"))

# 打 deb 包(Windows 上 zigbuild 交叉;依赖 openssl 预置,否则用 deb-native)
deb: build-linux gen-packaging
    cargo deb --no-build --no-strip --target {{target_short}}
    New-Item -ItemType Directory -Force -Path {{dist_dir}} | Out-Null
    Copy-Item target/debian/*.deb {{dist_dir}}/
    Get-ChildItem {{dist_dir}}/ | Select-Object Name, Length

# 打 deb 包(Linux 原生构建,推荐生产部署:在工控机/CI 执行)
deb-native: gen-packaging
    cargo build --{{profile}}
    cargo deb --no-build --no-strip
    New-Item -ItemType Directory -Force -Path {{dist_dir}} | Out-Null
    Copy-Item target/debian/*.deb {{dist_dir}}/
    Get-ChildItem {{dist_dir}}/ | Select-Object Name, Length

# 读取当前版本号(Cargo.toml 为唯一来源)
version:
    @(Select-String -Path Cargo.toml -Pattern '^version = "([^"]+)"' | Select-Object -First 1).Matches[0].Groups[1].Value

# 一键打包(Windows 包)
dist: build-win
    @$v = (just version | Select-Object -Last 1).Trim(); $d = '{{dist_dir}}/os-active-v' + $v; if (Test-Path $d) { Remove-Item -Recurse -Force $d }; New-Item -ItemType Directory -Force -Path ($d + '/windows') | Out-Null; Copy-Item {{win_exe}} ($d + '/windows/'); Copy-Item README.md $d; if (Test-Path LICENSE) { Copy-Item LICENSE $d }; $zip = '{{dist_dir}}/os-active-v' + $v + '.zip'; if (Test-Path $zip) { Remove-Item -Force $zip }; Compress-Archive -Path $d -DestinationPath $zip; Write-Host ('打包完成: ' + (Resolve-Path $zip).Path)

# 一键打包(Windows + Linux 双平台)
dist-all: build-win build-linux
    @$v = (just version | Select-Object -Last 1).Trim(); $d = '{{dist_dir}}/os-active-v' + $v; if (Test-Path $d) { Remove-Item -Recurse -Force $d }; New-Item -ItemType Directory -Force -Path ($d + '/windows'), ($d + '/linux') | Out-Null; Copy-Item {{win_exe}} ($d + '/windows/'); Copy-Item {{linux_bin}} ($d + '/linux/'); Copy-Item README.md $d; if (Test-Path LICENSE) { Copy-Item LICENSE $d }; $zip = '{{dist_dir}}/os-active-v' + $v + '.zip'; if (Test-Path $zip) { Remove-Item -Force $zip }; Compress-Archive -Path $d -DestinationPath $zip; Write-Host ('打包完成: ' + (Resolve-Path $zip).Path)

# ---- 开发机 VM 验证(环境特定,可选)----

# 启动 Ubuntu 18.04 验证 VM(仅本机 VMware)
vm-start:
    Start-Process "C:/Program Files/VMware/x64/vmware-vmx.exe" -ArgumentList '-x','-q','--','{{vm_vmx}}'

# 上传 Linux 二进制到验证 VM,Xvfb 无头运行并检查日志/存活(适配 os-active)
vm-run: build-linux
    {{vm_scp}} target/{{target_short}}/{{profile}}/{{bin}} {{vm_host}}:~/os-active
    {{vm_ssh}} {{vm_host}} "chmod +x os-active; pkill -x os-active; pkill -x Xvfb; sleep 1; \
      (Xvfb :99 -screen 0 1280x760x24 >/dev/null 2>&1 &); sleep 2; \
      (DISPLAY=:99 E_AUTOTEST_GL=egl ./os-active >/tmp/os-active.log 2>&1 </dev/null &); sleep 8; \
      pgrep -x os-active >/dev/null && echo OS-ACTIVE-ALIVE || echo OS-ACTIVE-DEAD; \
      tail -8 /tmp/os-active.log; pkill -x os-active; pkill -x Xvfb; true"

# deb 包在验证 VM 内安装回归
vm-install: deb
    {{vm_scp}} target/debian/*.deb {{vm_host}}:/tmp/
    {{vm_ssh}} {{vm_host}} "echo test123 | sudo -S dpkg -i /tmp/os-active_*.deb 2>&1 | tail -2; dpkg -l os-active | tail -1"


# 打包成文件夹部署包(参考 TP100:主程序 + plugins(架构分层) + config + README/LICENSE)
pack: build-win
    @$v = (just version | Select-Object -Last 1).Trim(); $d = '{{dist_dir}}/os-active-pack-v' + $v; if (Test-Path $d) { Remove-Item -Recurse -Force $d }
    @New-Item -ItemType Directory -Force -Path "$d/windows", "$d/tools/ByoDmi/x86_64/linux" | Out-Null
    @Copy-Item {{win_exe}} "$d/windows/"
    @Copy-Item tools/ByoDmi/x86_64/linux/* "$d/plugins/x86_64/ByoDmi/"
    @Copy-Item README.md, LICENSE $d
    @if (Test-Path os-active.toml) { Copy-Item os-active.toml $d }
    @Write-Host ('文件夹部署包: ' + (Resolve-Path $d).Path)
    @Get-ChildItem $d -Recurse -File | Select-Object FullName | Out-String
# 清理构建产物(含打包目录)
clean:
    cargo clean
    @if (Test-Path {{dist_dir}}) { Remove-Item -Recurse -Force {{dist_dir}} }