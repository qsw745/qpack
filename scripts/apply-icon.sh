#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CHOICE="${1:-a}"

ICON_A="$ROOT_DIR/assets/icon-source.png"

case "$CHOICE" in
  a|A)
    SRC_IMG="$ICON_A"
    CROP_INSET="0.0"
    echo ">>> 应用方案 A：珍珠白底 + 3D 磨砂玻璃折叠拉链盒"
    ;;
  *)
    SRC_IMG="$CHOICE"
    CROP_INSET="0.0"
    echo ">>> 应用自定义图标：$SRC_IMG"
    ;;
esac

if [ ! -f "$SRC_IMG" ]; then
  echo "错误：找不到源图片 $SRC_IMG"
  exit 1
fi

OUT_1024="$ROOT_DIR/src-tauri/icons/app-icon-1024.png"
PUBLIC_LOGO="$ROOT_DIR/public/logo.png"
mkdir -p "$ROOT_DIR/src-tauri/icons" "$ROOT_DIR/public"

# 使用 macOS 原生 CoreGraphics 精准裁切外围浅灰背景，并套用 Apple 标准 10% 透明安全边距与圆角遮罩
swift - "$SRC_IMG" "$OUT_1024" "$CROP_INSET" <<'EOF'
import Foundation
import AppKit
import CoreGraphics

let args = CommandLine.arguments
let srcPath = args[1]
let dstPath = args[2]
let insetRatio = CGFloat(Double(args[3]) ?? 0.16)

guard let nsImage = NSImage(contentsOfFile: srcPath),
      let cgImage = nsImage.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
    fputs("无法读取图片: \(srcPath)\n", stderr)
    exit(1)
}

let w = CGFloat(cgImage.width)
let h = CGFloat(cgImage.height)
let cropRect = CGRect(
    x: w * insetRatio,
    y: h * insetRatio,
    width: w * (1.0 - insetRatio * 2.0),
    height: h * (1.0 - insetRatio * 2.0)
)

guard let cropped = cgImage.cropping(to: cropRect) else {
    fputs("裁切图片失败\n", stderr)
    exit(1)
}

let canvasSize = 1024
let colorSpace = CGColorSpaceCreateDeviceRGB()
guard let ctx = CGContext(
    data: nil,
    width: canvasSize,
    height: canvasSize,
    bitsPerComponent: 8,
    bytesPerRow: 0,
    space: colorSpace,
    bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
) else {
    fputs("创建画布失败\n", stderr)
    exit(1)
}

ctx.interpolationQuality = .high
ctx.setAllowsAntialiasing(true)
ctx.setShouldAntialias(true)

// Apple macOS 图标规范：1024 画布内主体 824x824（四周约 10% 透明边距），圆角半径约 22.37%
let padding: CGFloat = 100.0
let iconSide: CGFloat = CGFloat(canvasSize) - padding * 2.0
let drawRect = CGRect(x: padding, y: padding, width: iconSide, height: iconSide)
let cornerRadius = iconSide * 0.2237
let clipPath = CGPath(roundedRect: drawRect, cornerWidth: cornerRadius, cornerHeight: cornerRadius, transform: nil)

// 绘制柔和自然投影
ctx.saveGState()
ctx.setShadow(offset: CGSize(width: 0, height: -10), blur: 24, color: CGColor(gray: 0.0, alpha: 0.22))
ctx.addPath(clipPath)
ctx.setFillColor(CGColor(gray: 1.0, alpha: 1.0))
ctx.fillPath()
ctx.restoreGState()

// 裁剪并绘制高清图标主体
ctx.saveGState()
ctx.addPath(clipPath)
ctx.clip()
ctx.draw(cropped, in: drawRect)
ctx.restoreGState()

guard let outImage = ctx.makeImage() else {
    fputs("生成输出图片失败\n", stderr)
    exit(1)
}

let bitmap = NSBitmapImageRep(cgImage: outImage)
guard let pngData = bitmap.representation(using: .png, properties: [:]) else {
    fputs("编码 PNG 失败\n", stderr)
    exit(1)
}

try pngData.write(to: URL(fileURLWithPath: dstPath))
print("已生成 1024x1024 透明规范图标: \(dstPath)")
EOF

cp "$OUT_1024" "$PUBLIC_LOGO"

echo ">>> 正在调用 Tauri CLI 生成全平台图标（PNG / macOS .icns / Windows .ico）..."
cd "$ROOT_DIR"
pnpm tauri icon "$OUT_1024"

echo ">>> 全部图标生成并替换完成！"
