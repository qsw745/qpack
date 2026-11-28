#!/usr/bin/env bash
set -euo pipefail

# QPack macOS 一键安装脚本
# 用法: curl -fsSL https://你的域名/install.sh | bash
# 可通过环境变量 QPACK_BASE_URL 指定镜像/服务器地址

BASE_URL="${QPACK_BASE_URL:-http://192.168.31.147:8080}"
BASE_URL="${BASE_URL%/}"
VERSION="${QPACK_VERSION:-0.1.7}"

ARCH="$(uname -m)"
case "$ARCH" in
  arm64|aarch64)
    DMG_NAME="QPack_${VERSION}_aarch64.dmg"
    ;;
  x86_64)
    DMG_NAME="QPack_${VERSION}_x64.dmg"
    ;;
  *)
    echo "错误：暂不支持当前 CPU 架构 ($ARCH)" >&2
    exit 1
    ;;
esac

DOWNLOAD_URL="${BASE_URL}/releases/${DMG_NAME}"
TMP_DIR="$(mktemp -d /tmp/qpack-install.XXXXXX)"
DMG_PATH="${TMP_DIR}/${DMG_NAME}"
MOUNT_POINT="${TMP_DIR}/mount"

cleanup() {
  if [ -d "$MOUNT_POINT" ]; then
    hdiutil detach "$MOUNT_POINT" -quiet >/dev/null 2>&1 || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

echo ">>> 正在下载 QPack v${VERSION} (${ARCH})..."
echo "    ${DOWNLOAD_URL}"
curl -fL --progress-bar "$DOWNLOAD_URL" -o "$DMG_PATH"

echo ">>> 正在挂载安装包..."
mkdir -p "$MOUNT_POINT"
hdiutil attach "$DMG_PATH" -nobrowse -readonly -mountpoint "$MOUNT_POINT" -quiet

if [ ! -d "${MOUNT_POINT}/QPack.app" ]; then
  echo "错误：DMG 中未找到 QPack.app" >&2
  exit 1
fi

echo ">>> 正在安装到 /Applications/QPack.app..."
rm -rf /Applications/QPack.app
cp -R "${MOUNT_POINT}/QPack.app" /Applications/QPack.app

# 清除下载隔离属性，避免首次打开提示未验证开发者
xattr -dr com.apple.quarantine /Applications/QPack.app >/dev/null 2>&1 || true

# 刷新 macOS 文件关联缓存，确保访达「打开方式」立即显示 QPack
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f /Applications/QPack.app >/dev/null 2>&1 || true

echo ""
echo "✅ QPack v${VERSION} 安装完成！"
echo "   可在「应用程序」或聚焦搜索（⌘Space）中打开 QPack。"
echo "   若需立刻启动，请运行: open -a QPack"
