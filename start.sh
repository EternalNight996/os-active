#!/bin/bash
# os-active 启动脚本
# 部署结构: start.sh 在包根,主程序在 linux/os-active(或同目录 os-active)
# 功能: 定位主程序 + root 提权(读 DMI SN) + 图形会话保留 + EGL 三级显示回退 + 输入法适配

ROOT_PATH=$(cd "$(dirname "$0")"; pwd)

# ---- 定位主程序:优先 linux/os-active(部署包),其次同目录 os-active ----
if [ -x "$ROOT_PATH/linux/os-active" ]; then
  BIN="$ROOT_PATH/linux/os-active"
elif [ -x "$ROOT_PATH/os-active" ]; then
  BIN="$ROOT_PATH/os-active"
else
  echo "错误: 未找到 os-active 主程序(期望 linux/os-active 或 os-active)"
  exit 1
fi
BIN_DIR=$(cd "$(dirname "$BIN")"; pwd)
LOG_DIR=$BIN_DIR/logs

# ---- 提权: 非 root 用 sudo 提权(root 才能读 DMI Serial) ----
if [ "$EUID" -ne 0 ] && [ "$1" != "--root-mode" ]; then
    ORIGINAL_USER="$USER"
    DISPLAY_ENV=""; [ -n "$DISPLAY" ] && DISPLAY_ENV="DISPLAY=$DISPLAY"
    XAUTH_ENV=""; [ -n "$XAUTHORITY" ] && XAUTH_ENV="XAUTHORITY=$XAUTHORITY"
    DBUS_ENV=""; [ -n "$DBUS_SESSION_BUS_ADDRESS" ] && DBUS_ENV="DBUS_SESSION_BUS_ADDRESS=$DBUS_SESSION_BUS_ADDRESS"
    WAYLAND_ENV=""; [ -n "$WAYLAND_DISPLAY" ] && WAYLAND_ENV="WAYLAND_DISPLAY=$WAYLAND_DISPLAY"
    XDG_ENV="XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
    if command -v sudo > /dev/null 2>&1; then
        echo "os-active 需要 root 权限读取 DMI SN,尝试提权..."
        sudo -n env $DISPLAY_ENV $XAUTH_ENV $DBUS_ENV $WAYLAND_ENV $XDG_ENV \
            ORIGINAL_USER="$ORIGINAL_USER" bash "$0" --root-mode "$@" 2>/dev/null && exit 0
        echo "警告: 免密 sudo 不可用,将以普通用户运行(SN 读取可能受限)"
    else
        echo "警告: 未找到 sudo,以普通用户运行"
    fi
fi

# 清理旧进程
pkill -f "$(basename "$BIN")" 2>/dev/null || true

# ---- 输入法检测(egui 中文输入) ----
DETECT_USER="${ORIGINAL_USER:-$USER}"
if pgrep -u "$DETECT_USER" ibus-daemon > /dev/null 2>&1; then
    export QT_IM_MODULE=ibus; export GTK_IM_MODULE=ibus; export XMODIFIERS="@im=ibus"
elif pgrep -u "$DETECT_USER" fcitx > /dev/null 2>&1; then
    export QT_IM_MODULE=fcitx; export GTK_IM_MODULE=fcitx; export XMODIFIERS="@im=fcitx"
fi

# 工作目录 = 主程序目录,日志落程序目录 logs
cd "$BIN_DIR" || exit 1
mkdir -p "$LOG_DIR"

# ---- EGL 三级显示回退(国产 OS GLX 假成功崩溃) ----
run() { "$@"; rc=$?; [ "$rc" -eq 0 ] || [ "$rc" -eq 130 ]; }
run "$BIN" "$@" && exit 0
echo "[os-active] GLX 模式失败(exit $rc),尝试 EGL..."
run env E_AUTOTEST_GL=egl "$BIN" "$@" && exit 0
echo "[os-active] EGL 模式失败(exit $rc),尝试 EGL + 软件渲染..."
run env E_AUTOTEST_GL=egl LIBGL_ALWAYS_SOFTWARE=1 "$BIN" "$@" && exit 0
echo "[os-active] 所有显示后端均失败,请检查日志: $LOG_DIR"
exit 1
