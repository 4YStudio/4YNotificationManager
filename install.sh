#!/usr/bin/env bash
# 4Y 通知提示音管理器 — 一键安装脚本
# 从 GitHub Releases 下载最新 .deb 并安装，然后自动启用后台服务。
#
# 用法:
#   curl -fsSL https://raw.githubusercontent.com/4YStudio/4YNotificationManager/main/install.sh | bash
#
set -euo pipefail

REPO="4YStudio/4YNotificationManager"
API="https://api.github.com/repos/${REPO}/releases/latest"

info() { printf '\033[1;34m[4Y]\033[0m %s\n' "$*"; }
err()  { printf '\033[1;31m[4Y]\033[0m %s\n' "$*" >&2; }

# 环境检查
if [[ "$(uname -s)" != "Linux" ]]; then
    err "仅支持 Linux (Debian 13)"
    exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
    err "需要 curl"
    exit 1
fi

info "查询最新版本…"
URL="$(curl -fsSL "$API" | grep -oE '"browser_download_url":[[:space:]]*"[^"]+\.deb"' | head -1 | sed -E 's/.*"(https[^"]+)"/\1/')"

if [[ -z "${URL}" ]]; then
    err "未找到 .deb 发行包。请先到 https://github.com/${REPO}/releases 手动下载。"
    exit 1
fi

info "下载: ${URL}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
curl -fL --progress-bar "$URL" -o "$TMP/app.deb"

info "安装 (需要 sudo)…"
sudo apt-get install -y "$TMP/app.deb" || sudo dpkg -i "$TMP/app.deb" || sudo apt-get -f install -y

info "启用后台服务…"
if command -v fourey-notification-manager >/dev/null 2>&1; then
    fourey-notification-manager --install-service
else
    info "请从应用菜单打开「4Y 通知提示音」并点击「一键启用后台服务」"
fi

info "完成 ✅  现在打开应用即可为不同软件配置提示音。"
