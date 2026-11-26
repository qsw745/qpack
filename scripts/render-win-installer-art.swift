import Foundation
import AppKit
import CoreGraphics

let rootDir = FileManager.default.currentDirectoryPath
let logoPath = "\(rootDir)/public/logo.png"
let outDir = "\(rootDir)/src-tauri/icons"

guard let logoNS = NSImage(contentsOfFile: logoPath),
      let logoCG = logoNS.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
    fputs("无法读取 Logo: \(logoPath)\n", stderr)
    exit(1)
}

let cs = CGColorSpaceCreateDeviceRGB()

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

func makeCanvas(width: Int, height: Int, draw: (CGContext) -> Void, savePNG: String) {
    guard let ctx = CGContext(
        data: nil,
        width: width,
        height: height,
        bitsPerComponent: 8,
        bytesPerRow: 0,
        space: cs,
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    ) else {
        fputs("创建画布失败\n", stderr)
        exit(1)
    }
    ctx.interpolationQuality = .high
    ctx.setAllowsAntialiasing(true)
    ctx.setShouldAntialias(true)
    ctx.setAllowsFontSmoothing(true)
    ctx.setShouldSmoothFonts(true)

    let nsCtx = NSGraphicsContext(cgContext: ctx, flipped: true)
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = nsCtx
    ctx.translateBy(x: 0, y: CGFloat(height))
    ctx.scaleBy(x: 1, y: -1)

    draw(ctx)

    NSGraphicsContext.restoreGraphicsState()

    guard let img = ctx.makeImage() else { exit(1) }
    let rep = NSBitmapImageRep(cgImage: img)
    guard let png = rep.representation(using: .png, properties: [:]) else { exit(1) }
    try! png.write(to: URL(fileURLWithPath: savePNG))
    print("已生成: \(savePNG)")
}

func drawLinear(_ ctx: CGContext, _ colors: [CGColor], _ locs: [CGFloat], _ start: CGPoint, _ end: CGPoint) {
    if let g = CGGradient(colorsSpace: cs, colors: colors as CFArray, locations: locs) {
        ctx.drawLinearGradient(g, start: start, end: end, options: [.drawsBeforeStartLocation, .drawsAfterEndLocation])
    }
}

func drawRadial(_ ctx: CGContext, _ colors: [CGColor], _ locs: [CGFloat], _ center: CGPoint, _ radius: CGFloat) {
    if let g = CGGradient(colorsSpace: cs, colors: colors as CFArray, locations: locs) {
        ctx.drawRadialGradient(g, startCenter: center, startRadius: 0, endCenter: center, endRadius: radius, options: [.drawsAfterEndLocation])
    }
}

func drawLogo(_ ctx: CGContext, in rect: CGRect) {
    ctx.saveGState()
    // 翻转回正向绘制图片
    ctx.translateBy(x: rect.minX, y: rect.maxY)
    ctx.scaleBy(x: 1, y: -1)
    ctx.draw(logoCG, in: CGRect(x: 0, y: 0, width: rect.width, height: rect.height))
    ctx.restoreGState()
}

func drawSidebarPanel(_ ctx: CGContext, width: CGFloat, height: CGFloat) {
    // 蓝青磨砂渐变侧栏背景（与 QPack 3D 磨砂玻璃图标呼应）
    ctx.saveGState()
    ctx.clip(to: CGRect(x: 0, y: 0, width: width, height: height))
    drawLinear(
        ctx,
        [color(0x1D4ED8), color(0x2563EB), color(0x0EA5E9)],
        [0.0, 0.52, 1.0],
        CGPoint(x: 0, y: 0),
        CGPoint(x: width, y: height)
    )
    // 柔和顶部光晕
    drawRadial(
        ctx,
        [color(0xFFFFFF, 0.34), color(0x7DD3FC, 0.12), color(0x7DD3FC, 0.0)],
        [0.0, 0.55, 1.0],
        CGPoint(x: width * 0.5, y: 68),
        130
    )
    // 底部微亮装饰玻璃圆弧
    drawRadial(
        ctx,
        [color(0x38BDF8, 0.35), color(0x38BDF8, 0.0)],
        [0.0, 1.0],
        CGPoint(x: width * 0.9, y: height * 0.95),
        120
    )

    // 居中圆角白色光环底座 + QPack Logo
    let logoSize: CGFloat = 92
    let logoX = (width - logoSize) / 2
    let logoY: CGFloat = 52
    drawLogo(ctx, in: CGRect(x: logoX, y: logoY, width: logoSize, height: logoSize))

    // 品牌名与副标题
    let para = NSMutableParagraphStyle()
    para.alignment = .center
    let titleAttrs: [NSAttributedString.Key: Any] = [
        .font: NSFont.systemFont(ofSize: 21, weight: .bold),
        .foregroundColor: NSColor.white,
        .paragraphStyle: para
    ]
    NSAttributedString(string: "QPack", attributes: titleAttrs)
        .draw(in: CGRect(x: 0, y: 154, width: width, height: 30))

    let subAttrs: [NSAttributedString.Key: Any] = [
        .font: NSFont.systemFont(ofSize: 10.5, weight: .medium),
        .foregroundColor: nsColor(0xE0F2FE, 0.92),
        .paragraphStyle: para
    ]
    NSAttributedString(string: "极简 · 高性能\n跨平台解压缩工具", attributes: subAttrs)
        .draw(in: CGRect(x: 10, y: 186, width: width - 20, height: 38))

    // 底部版本胶囊
    let pillW: CGFloat = 78
    let pillH: CGFloat = 20
    let pillRect = CGRect(x: (width - pillW) / 2, y: height - 38, width: pillW, height: pillH)
    let pillPath = CGPath(roundedRect: pillRect, cornerWidth: 10, cornerHeight: 10, transform: nil)
    ctx.addPath(pillPath)
    ctx.setFillColor(color(0xFFFFFF, 0.18))
    ctx.fillPath()
    ctx.addPath(pillPath)
    ctx.setStrokeColor(color(0xFFFFFF, 0.35))
    ctx.setLineWidth(1.0)
    ctx.strokePath()

    let verAttrs: [NSAttributedString.Key: Any] = [
        .font: NSFont.systemFont(ofSize: 9.5, weight: .semibold),
        .foregroundColor: NSColor.white,
        .paragraphStyle: para
    ]
    NSAttributedString(string: "v0.1.0", attributes: verAttrs)
        .draw(in: CGRect(x: pillRect.minX, y: pillRect.minY + 3.5, width: pillW, height: 16))
    ctx.restoreGState()
}

// 1. NSIS Sidebar (164 x 314)
makeCanvas(
    width: 164,
    height: 314,
    draw: { ctx in
        drawSidebarPanel(ctx, width: 164, height: 314)
    },
    savePNG: "/tmp/win-nsis-sidebar.png"
)

// 2. NSIS Header (150 x 57) — 顶部右侧简洁珍珠白 + 蓝青 Logo 徽标
makeCanvas(
    width: 150,
    height: 57,
    draw: { ctx in
        drawLinear(
            ctx,
            [color(0xFFFFFF), color(0xF1F6FE)],
            [0.0, 1.0],
            CGPoint(x: 0, y: 0),
            CGPoint(x: 150, y: 57)
        )
        drawLogo(ctx, in: CGRect(x: 92, y: 5, width: 47, height: 47))
        let attrs: [NSAttributedString.Key: Any] = [
            .font: NSFont.systemFont(ofSize: 15, weight: .bold),
            .foregroundColor: nsColor(0x1E3A8A, 0.92)
        ]
        NSAttributedString(string: "QPack", attributes: attrs)
            .draw(in: CGRect(x: 34, y: 18, width: 60, height: 24))
    },
    savePNG: "/tmp/win-nsis-header.png"
)

// 3. WiX Dialog (493 x 312) — 左侧 164px 视觉栏 + 右侧纯白文字区
makeCanvas(
    width: 493,
    height: 312,
    draw: { ctx in
        ctx.setFillColor(color(0xFFFFFF))
        ctx.fill(CGRect(x: 0, y: 0, width: 493, height: 312))
        drawSidebarPanel(ctx, width: 164, height: 312)
        // 分隔细线
        ctx.setFillColor(color(0xE2E8F0))
        ctx.fill(CGRect(x: 164, y: 0, width: 1, height: 312))
    },
    savePNG: "/tmp/win-wix-dialog.png"
)

// 4. WiX Banner (493 x 58) — 左侧留白给向导标题，右侧展示 QPack 图标
makeCanvas(
    width: 493,
    height: 58,
    draw: { ctx in
        drawLinear(
            ctx,
            [color(0xFFFFFF), color(0xF8FBFF), color(0xEDF4FD)],
            [0.0, 0.72, 1.0],
            CGPoint(x: 0, y: 0),
            CGPoint(x: 493, y: 0)
        )
        drawLogo(ctx, in: CGRect(x: 438, y: 5, width: 46, height: 46))
        ctx.setFillColor(color(0xE2E8F0))
        ctx.fill(CGRect(x: 0, y: 57, width: 493, height: 1))
    },
    savePNG: "/tmp/win-wix-banner.png"
)
