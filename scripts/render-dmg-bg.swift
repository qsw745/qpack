import Foundation
import AppKit
import CoreGraphics

// 逻辑窗口尺寸 660x420，导出 2x Retina 1320x840 以确保在 macOS 上极度清晰
let logicalW: CGFloat = 660
let logicalH: CGFloat = 420
let scale: CGFloat = 2.0
let pixelW = Int(logicalW * scale)
let pixelH = Int(logicalH * scale)

let cs = CGColorSpaceCreateDeviceRGB()
guard let ctx = CGContext(
    data: nil,
    width: pixelW,
    height: pixelH,
    bitsPerComponent: 8,
    bytesPerRow: 0,
    space: cs,
    bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
) else {
    fputs("无法创建画布\n", stderr)
    exit(1)
}

ctx.interpolationQuality = .high
ctx.setAllowsAntialiasing(true)
ctx.setShouldAntialias(true)
ctx.setAllowsFontSmoothing(true)
ctx.setShouldSmoothFonts(true)

// 使用 NSGraphicsContext 支持高质量中英文字体渲染
let nsCtx = NSGraphicsContext(cgContext: ctx, flipped: true)
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = nsCtx

// 坐标系转换：左上角为原点，按逻辑坐标 660x420 绘制
ctx.translateBy(x: 0, y: CGFloat(pixelH))
ctx.scaleBy(x: scale, y: -scale)

func color(_ hex: UInt32, _ alpha: CGFloat = 1.0) -> CGColor {
    let r = CGFloat((hex >> 16) & 0xFF) / 255.0
    let g = CGFloat((hex >> 8) & 0xFF) / 255.0
    let b = CGFloat(hex & 0xFF) / 255.0
    return CGColor(colorSpace: cs, components: [r, g, b, alpha])!
}

func nsColor(_ hex: UInt32, _ alpha: CGFloat = 1.0) -> NSColor {
    let r = CGFloat((hex >> 16) & 0xFF) / 255.0
    let g = CGFloat((hex >> 8) & 0xFF) / 255.0
    let b = CGFloat(hex & 0xFF) / 255.0
    return NSColor(calibratedRed: r, green: g, blue: b, alpha: alpha)
}

func drawLinearGradient(_ colors: [CGColor], _ locs: [CGFloat], start: CGPoint, end: CGPoint) {
    if let g = CGGradient(colorsSpace: cs, colors: colors as CFArray, locations: locs) {
        ctx.drawLinearGradient(g, start: start, end: end, options: [.drawsBeforeStartLocation, .drawsAfterEndLocation])
    }
}

func drawRadialGradient(_ colors: [CGColor], _ locs: [CGFloat], center: CGPoint, radius: CGFloat) {
    if let g = CGGradient(colorsSpace: cs, colors: colors as CFArray, locations: locs) {
        ctx.drawRadialGradient(g, startCenter: center, startRadius: 0, endCenter: center, endRadius: radius, options: [.drawsAfterEndLocation])
    }
}

// 1. 柔和珍珠白至冰蓝工作室背景
drawLinearGradient(
    [color(0xFFFFFF), color(0xF6F9FF), color(0xEAF2FC)],
    [0.0, 0.55, 1.0],
    start: CGPoint(x: 0, y: 0),
    end: CGPoint(x: logicalW, y: logicalH)
)

// 顶部柔和蓝青环境光晕
drawRadialGradient(
    [color(0x38BDF8, 0.14), color(0x3B82F6, 0.06), color(0x3B82F6, 0.0)],
    [0.0, 0.55, 1.0],
    center: CGPoint(x: 170, y: 60),
    radius: 280
)
drawRadialGradient(
    [color(0x60A5FA, 0.12), color(0x0EA5E9, 0.05), color(0x0EA5E9, 0.0)],
    [0.0, 0.55, 1.0],
    center: CGPoint(x: 500, y: 350),
    radius: 260
)

// 2. 细微装饰点阵网格（增加高级感）
ctx.saveGState()
ctx.setFillColor(color(0x94A3B8, 0.14))
for row in stride(from: 24.0, to: Double(logicalH) - 16.0, by: 24.0) {
    for col in stride(from: 24.0, to: Double(logicalW) - 16.0, by: 24.0) {
        ctx.fillEllipse(in: CGRect(x: col, y: row, width: 1.5, height: 1.5))
    }
}
ctx.restoreGState()

// 3. 中央磨砂玻璃承载卡片
let cardRect = CGRect(x: 42, y: 112, width: 576, height: 222)
let cardPath = CGPath(roundedRect: cardRect, cornerWidth: 26, cornerHeight: 26, transform: nil)
ctx.saveGState()
ctx.setShadow(offset: CGSize(width: 0, height: -8), blur: 24, color: color(0x1E3A8A, 0.08))
ctx.addPath(cardPath)
ctx.setFillColor(color(0xFFFFFF, 0.78))
ctx.fillPath()
ctx.restoreGState()

ctx.saveGState()
ctx.addPath(cardPath)
ctx.setStrokeColor(color(0xFFFFFF, 0.95))
ctx.setLineWidth(1.5)
ctx.strokePath()
ctx.restoreGState()

// 4. 左右两个图标对齐光环托盘（精准对准 appPosition=(180, 215) 和 applicationFolderPosition=(480, 215)）
func drawDropSlot(center: CGPoint, dashed: Bool) {
    let slotSize: CGFloat = 136
    let rect = CGRect(x: center.x - slotSize / 2, y: center.y - slotSize / 2, width: slotSize, height: slotSize)
    let path = CGPath(roundedRect: rect, cornerWidth: 28, cornerHeight: 28, transform: nil)

    ctx.saveGState()
    ctx.addPath(path)
    ctx.setFillColor(color(0xEFF6FF, dashed ? 0.55 : 0.45))
    ctx.fillPath()
    if dashed {
        ctx.setLineDash(phase: 0, lengths: [6, 5])
        ctx.setStrokeColor(color(0x3B82F6, 0.42))
    } else {
        ctx.setStrokeColor(color(0xCBD5E1, 0.55))
    }
    ctx.setLineWidth(1.6)
    ctx.addPath(path)
    ctx.strokePath()
    ctx.restoreGState()
}

drawDropSlot(center: CGPoint(x: 180, y: 215), dashed: false)
drawDropSlot(center: CGPoint(x: 480, y: 215), dashed: true)

// 5. 中央蓝青渐变引导箭头（从左侧 QPack 指向右侧 Applications）
ctx.saveGState()
let arrowY: CGFloat = 215
let lineStartX: CGFloat = 278
let lineEndX: CGFloat = 376

// 箭头柔光底层
let pillRect = CGRect(x: 266, y: arrowY - 22, width: 128, height: 44)
let pillPath = CGPath(roundedRect: pillRect, cornerWidth: 22, cornerHeight: 22, transform: nil)
ctx.saveGState()
ctx.addPath(pillPath)
ctx.setFillColor(color(0xEFF6FF, 0.9))
ctx.fillPath()
ctx.addPath(pillPath)
ctx.setStrokeColor(color(0xBFDBFE, 0.8))
ctx.setLineWidth(1.2)
ctx.strokePath()
ctx.restoreGState()

// 箭头主体渐变线条
ctx.saveGState()
ctx.setLineCap(.round)
ctx.setLineJoin(.round)
ctx.setLineWidth(4.0)
ctx.setStrokeColor(color(0x2563EB, 0.92))
ctx.move(to: CGPoint(x: lineStartX, y: arrowY))
ctx.addLine(to: CGPoint(x: lineEndX, y: arrowY))
ctx.strokePath()

ctx.move(to: CGPoint(x: lineEndX - 12, y: arrowY - 10))
ctx.addLine(to: CGPoint(x: lineEndX + 2, y: arrowY))
ctx.addLine(to: CGPoint(x: lineEndX - 12, y: arrowY + 10))
ctx.strokePath()
ctx.restoreGState()
ctx.restoreGState()

// 6. 顶部品牌标题与安装引导文字
func drawCenteredText(_ text: String, y: CGFloat, font: NSFont, textColor: NSColor) {
    let para = NSMutableParagraphStyle()
    para.alignment = .center
    let attrs: [NSAttributedString.Key: Any] = [
        .font: font,
        .foregroundColor: textColor,
        .paragraphStyle: para
    ]
    let str = NSAttributedString(string: text, attributes: attrs)
    str.draw(in: CGRect(x: 0, y: y, width: logicalW, height: 44))
}

let titleFont = NSFont.systemFont(ofSize: 28, weight: .bold)
let subFont = NSFont.systemFont(ofSize: 13.5, weight: .medium)
let footerFont = NSFont.systemFont(ofSize: 11.5, weight: .regular)

drawCenteredText("QPack", y: 28, font: titleFont, textColor: nsColor(0x0F172A, 0.94))
drawCenteredText("将 QPack 拖入右侧「应用程序」文件夹即可完成安装", y: 68, font: subFont, textColor: nsColor(0x475569, 0.92))
drawCenteredText("极简 · 高性能 · 跨平台解压缩工具", y: 362, font: footerFont, textColor: nsColor(0x64748B, 0.85))

NSGraphicsContext.restoreGraphicsState()

guard let img = ctx.makeImage() else {
    fputs("生成 DMG 背景图失败\n", stderr)
    exit(1)
}
let rep = NSBitmapImageRep(cgImage: img)
guard let pngData = rep.representation(using: .png, properties: [:]) else {
    fputs("编码 PNG 失败\n", stderr)
    exit(1)
}

let outPath = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "src-tauri/icons/dmg-background.png"
try pngData.write(to: URL(fileURLWithPath: outPath))
print("已生成 Retina DMG 安装界面背景图: \(outPath)")
