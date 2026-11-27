import Foundation
import AppKit
import CoreGraphics

let root = FileManager.default.currentDirectoryPath
let outDir = "\(root)/packaging/store-listing"
try FileManager.default.createDirectory(atPath: outDir, withIntermediateDirectories: true)

let logoPath = "\(root)/public/logo.png"
let winIconPath = "\(root)/src-tauri/icons/win-taskbar-128.png"
guard let logoNS = NSImage(contentsOfFile: logoPath),
      let logoCG = logoNS.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
    fputs("无法读取 Logo\n", stderr)
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

func drawImage(_ ctx: CGContext, _ img: CGImage, in rect: CGRect) {
    ctx.saveGState()
    ctx.translateBy(x: rect.minX, y: rect.maxY)
    ctx.scaleBy(x: 1, y: -1)
    ctx.draw(img, in: CGRect(x: 0, y: 0, width: rect.width, height: rect.height))
    ctx.restoreGState()
}

func drawText(_ text: String, rect: CGRect, font: NSFont, textColor: NSColor, align: NSTextAlignment = .left) {
    let para = NSMutableParagraphStyle()
    para.alignment = align
    para.lineSpacing = 6
    let attrs: [NSAttributedString.Key: Any] = [
        .font: font,
        .foregroundColor: textColor,
        .paragraphStyle: para
    ]
    NSAttributedString(string: text, attributes: attrs).draw(in: rect)
}

func renderPNG(width: Int, height: Int, path: String, draw: (CGContext) -> Void) {
    guard let ctx = CGContext(
        data: nil, width: width, height: height,
        bitsPerComponent: 8, bytesPerRow: 0, space: cs,
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    ) else { exit(1) }
    ctx.interpolationQuality = .high
    ctx.setAllowsAntialiasing(true)
    ctx.setShouldAntialias(true)

    let nsCtx = NSGraphicsContext(cgContext: ctx, flipped: true)
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = nsCtx
    ctx.translateBy(x: 0, y: CGFloat(height))
    ctx.scaleBy(x: 1, y: -1)

    draw(ctx)

    NSGraphicsContext.restoreGraphicsState()
    guard let img = ctx.makeImage(),
          let png = NSBitmapImageRep(cgImage: img).representation(using: .png, properties: [:]) else { exit(1) }
    try! png.write(to: URL(fileURLWithPath: path))
    print("已生成: \(path)")
}

// 1. 300x300 Microsoft Store 徽标
renderPNG(width: 300, height: 300, path: "\(outDir)/store-icon-300x300.png") { ctx in
    drawImage(ctx, logoCG, in: CGRect(x: 0, y: 0, width: 300, height: 300))
}

func drawBackdrop(_ ctx: CGContext) {
    drawLinear(ctx, [color(0xFFFFFF), color(0xF3F7FE), color(0xE6F0FD)], [0.0, 0.55, 1.0], CGPoint(x: 0, y: 0), CGPoint(x: 1920, y: 1080))
    drawRadial(ctx, [color(0x3B82F6, 0.14), color(0x3B82F6, 0.0)], [0.0, 1.0], CGPoint(x: 280, y: 180), 540)
    drawRadial(ctx, [color(0x0EA5E9, 0.14), color(0x0EA5E9, 0.0)], [0.0, 1.0], CGPoint(x: 1640, y: 900), 560)
}

// 2. 商店宣传截图 1 (1920x1080): 品牌亮点 + 拖拽即用首页展示
renderPNG(width: 1920, height: 1080, path: "\(outDir)/screenshot-1-home.png") { ctx in
    drawBackdrop(ctx)

    // 顶部居中标题
    drawImage(ctx, logoCG, in: CGRect(x: 120, y: 92, width: 88, height: 88))
    drawText("QPack — 极简、高性能解压缩工具", rect: CGRect(x: 226, y: 102, width: 1200, height: 54), font: .systemFont(ofSize: 42, weight: .bold), textColor: nsColor(0x0F172A))
    drawText("智能防套娃解压 · 跨平台中文防乱码 · Windows 11 原生一级右键菜单 · 零广告", rect: CGRect(x: 226, y: 156, width: 1400, height: 36), font: .systemFont(ofSize: 21, weight: .medium), textColor: nsColor(0x475569))

    // 中央桌面应用窗口模拟
    let winRect = CGRect(x: 120, y: 236, width: 1680, height: 740)
    let winPath = CGPath(roundedRect: winRect, cornerWidth: 28, cornerHeight: 28, transform: nil)
    ctx.saveGState()
    ctx.setShadow(offset: CGSize(width: 0, height: -22), blur: 52, color: color(0x1E3A8A, 0.14))
    ctx.addPath(winPath)
    ctx.setFillColor(color(0xFFFFFF))
    ctx.fillPath()
    ctx.restoreGState()

    ctx.saveGState()
    ctx.addPath(winPath)
    ctx.setStrokeColor(color(0xCBD5E1, 0.7))
    ctx.setLineWidth(2)
    ctx.strokePath()
    ctx.restoreGState()

    // 窗口顶栏
    let barRect = CGRect(x: 120, y: 236, width: 1680, height: 74)
    ctx.saveGState()
    ctx.addPath(winPath)
    ctx.clip()
    ctx.setFillColor(color(0xF8FAFC))
    ctx.fill(barRect)
    ctx.setFillColor(color(0xE2E8F0))
    ctx.fill(CGRect(x: 120, y: 310, width: 1680, height: 1.5))
    ctx.restoreGState()

    drawImage(ctx, logoCG, in: CGRect(x: 152, y: 251, width: 44, height: 44))
    drawText("QPack", rect: CGRect(x: 208, y: 259, width: 200, height: 32), font: .systemFont(ofSize: 22, weight: .bold), textColor: nsColor(0x0F172A))

    // 居中拖拽虚线区
    let dropRect = CGRect(x: 260, y: 374, width: 1400, height: 390)
    let dropPath = CGPath(roundedRect: dropRect, cornerWidth: 26, cornerHeight: 26, transform: nil)
    ctx.saveGState()
    ctx.addPath(dropPath)
    ctx.setFillColor(color(0xF8FAFF))
    ctx.fillPath()
    ctx.setLineDash(phase: 0, lengths: [10, 8])
    ctx.setStrokeColor(color(0x93C5FD, 0.9))
    ctx.setLineWidth(2.5)
    ctx.addPath(dropPath)
    ctx.strokePath()
    ctx.restoreGState()

    drawImage(ctx, logoCG, in: CGRect(x: 904, y: 424, width: 112, height: 112))
    drawText("拖入压缩包即可预览或智能解压", rect: CGRect(x: 260, y: 556, width: 1400, height: 42), font: .systemFont(ofSize: 30, weight: .bold), textColor: nsColor(0x0F172A), align: .center)
    drawText("也可拖入任意文件或文件夹，一键打包为 ZIP / 7Z / TAR.ZST", rect: CGRect(x: 260, y: 608, width: 1400, height: 34), font: .systemFont(ofSize: 20, weight: .regular), textColor: nsColor(0x64748B), align: .center)

    // 两个主按钮
    let btn1 = CGRect(x: 690, y: 670, width: 240, height: 56)
    ctx.addPath(CGPath(roundedRect: btn1, cornerWidth: 14, cornerHeight: 14, transform: nil))
    ctx.setFillColor(color(0x2563EB))
    ctx.fillPath()
    drawText("打开压缩包", rect: CGRect(x: 690, y: 684, width: 240, height: 30), font: .systemFont(ofSize: 20, weight: .semibold), textColor: .white, align: .center)

    let btn2 = CGRect(x: 960, y: 670, width: 240, height: 56)
    let btn2Path = CGPath(roundedRect: btn2, cornerWidth: 14, cornerHeight: 14, transform: nil)
    ctx.addPath(btn2Path)
    ctx.setFillColor(color(0xFFFFFF))
    ctx.fillPath()
    ctx.addPath(btn2Path)
    ctx.setStrokeColor(color(0xCBD5E1))
    ctx.setLineWidth(2)
    ctx.strokePath()
    drawText("新建压缩包", rect: CGRect(x: 960, y: 684, width: 240, height: 30), font: .systemFont(ofSize: 20, weight: .semibold), textColor: nsColor(0x0F172A), align: .center)

    // 底部 3 个特性卡片
    let cards = [
        ("智能防套娃", "自动判断顶层结构，杜绝解压炸弹"),
        ("中文防乱码", "自动识别 GBK / UTF-8，一键切换编码"),
        ("Rust 原生双引擎", "高速处理 ZIP / 7Z / TAR / ZST / RAR")
    ]
    for (idx, item) in cards.enumerated() {
        let x = 260.0 + CGFloat(idx) * 480.0
        let r = CGRect(x: x, y: 804, width: 440, height: 120)
        let p = CGPath(roundedRect: r, cornerWidth: 18, cornerHeight: 18, transform: nil)
        ctx.addPath(p)
        ctx.setFillColor(color(0xF8FAFC))
        ctx.fillPath()
        ctx.addPath(p)
        ctx.setStrokeColor(color(0xE2E8F0))
        ctx.setLineWidth(1.5)
        ctx.strokePath()
        drawText(item.0, rect: CGRect(x: x + 26, y: 826, width: 390, height: 32), font: .systemFont(ofSize: 21, weight: .bold), textColor: nsColor(0x0F172A))
        drawText(item.1, rect: CGRect(x: x + 26, y: 866, width: 390, height: 30), font: .systemFont(ofSize: 16, weight: .regular), textColor: nsColor(0x64748B))
    }
}

// 3. 商店宣传截图 2 (1920x1080): 压缩包预览 + 实时解压进度 + 冲突智能处理
renderPNG(width: 1920, height: 1080, path: "\(outDir)/screenshot-2-browser.png") { ctx in
    drawBackdrop(ctx)

    drawImage(ctx, logoCG, in: CGRect(x: 120, y: 92, width: 88, height: 88))
    drawText("极速预览、实时进度与智能冲突处理", rect: CGRect(x: 226, y: 102, width: 1300, height: 54), font: .systemFont(ofSize: 42, weight: .bold), textColor: nsColor(0x0F172A))
    drawText("支持数万文件虚拟滚动浏览 · 应用内自定义右键菜单 · 同名文件自动新建目录防覆盖", rect: CGRect(x: 226, y: 156, width: 1500, height: 36), font: .systemFont(ofSize: 21, weight: .medium), textColor: nsColor(0x475569))

    let winRect = CGRect(x: 120, y: 236, width: 1680, height: 740)
    let winPath = CGPath(roundedRect: winRect, cornerWidth: 28, cornerHeight: 28, transform: nil)
    ctx.saveGState()
    ctx.setShadow(offset: CGSize(width: 0, height: -22), blur: 52, color: color(0x1E3A8A, 0.14))
    ctx.addPath(winPath)
    ctx.setFillColor(color(0xFFFFFF))
    ctx.fillPath()
    ctx.restoreGState()

    // 顶栏
    ctx.saveGState()
    ctx.addPath(winPath)
    ctx.clip()
    ctx.setFillColor(color(0xF8FAFC))
    ctx.fill(CGRect(x: 120, y: 236, width: 1680, height: 78))
    ctx.setFillColor(color(0xE2E8F0))
    ctx.fill(CGRect(x: 120, y: 314, width: 1680, height: 1.5))
    ctx.restoreGState()

    drawImage(ctx, logoCG, in: CGRect(x: 152, y: 253, width: 44, height: 44))
    drawText("QPack  /  2026项目交付资料包.zip", rect: CGRect(x: 210, y: 261, width: 700, height: 32), font: .systemFont(ofSize: 21, weight: .semibold), textColor: nsColor(0x0F172A))

    // 右侧工具按钮
    let smartBtn = CGRect(x: 1560, y: 253, width: 200, height: 44)
    ctx.addPath(CGPath(roundedRect: smartBtn, cornerWidth: 11, cornerHeight: 11, transform: nil))
    ctx.setFillColor(color(0x2563EB))
    ctx.fillPath()
    drawText("智能解压", rect: CGRect(x: 1560, y: 263, width: 200, height: 26), font: .systemFont(ofSize: 18, weight: .semibold), textColor: .white, align: .center)

    // 文件列表表头与行
    let rows = [
        ("设计规范与图标源文件/", "128.4 MB", "2026-11-24 18:10", "文件夹"),
        ("产品发布说明_终稿.pdf", "6.2 MB", "2026-11-24 19:22", "PDF 文档"),
        ("主视觉海报_4K.png", "18.9 MB", "2026-11-25 11:08", "PNG 图像"),
        ("财务预算汇总表.xlsx", "2.1 MB", "2026-11-25 14:15", "AES-256 加密"),
        ("Windows_x64_Release.bin", "42.7 MB", "2026-11-26 09:40", "二进制文件"),
    ]
    drawText("名称", rect: CGRect(x: 180, y: 342, width: 500, height: 26), font: .systemFont(ofSize: 17, weight: .semibold), textColor: nsColor(0x64748B))
    drawText("大小", rect: CGRect(x: 1020, y: 342, width: 180, height: 26), font: .systemFont(ofSize: 17, weight: .semibold), textColor: nsColor(0x64748B), align: .right)
    drawText("修改时间", rect: CGRect(x: 1240, y: 342, width: 240, height: 26), font: .systemFont(ofSize: 17, weight: .semibold), textColor: nsColor(0x64748B), align: .right)
    drawText("类型", rect: CGRect(x: 1520, y: 342, width: 220, height: 26), font: .systemFont(ofSize: 17, weight: .semibold), textColor: nsColor(0x64748B), align: .right)

    for (i, r) in rows.enumerated() {
        let y = 392.0 + CGFloat(i) * 72.0
        if i == 1 {
            ctx.addPath(CGPath(roundedRect: CGRect(x: 152, y: y - 10, width: 1616, height: 60), cornerWidth: 12, cornerHeight: 12, transform: nil))
            ctx.setFillColor(color(0xEFF6FF))
            ctx.fillPath()
        }
        drawText(r.0, rect: CGRect(x: 180, y: y + 6, width: 780, height: 30), font: .systemFont(ofSize: 20, weight: .medium), textColor: nsColor(0x0F172A))
        drawText(r.1, rect: CGRect(x: 1020, y: y + 6, width: 180, height: 30), font: .monospacedSystemFont(ofSize: 18, weight: .regular), textColor: nsColor(0x475569), align: .right)
        drawText(r.2, rect: CGRect(x: 1240, y: y + 6, width: 240, height: 30), font: .monospacedSystemFont(ofSize: 18, weight: .regular), textColor: nsColor(0x94A3B8), align: .right)
        drawText(r.3, rect: CGRect(x: 1520, y: y + 6, width: 220, height: 30), font: .systemFont(ofSize: 18, weight: .regular), textColor: nsColor(0x64748B), align: .right)
    }

    // 浮层居中实时解压进度卡片（展示核心亮点）
    let cardRect = CGRect(x: 610, y: 740, width: 700, height: 186)
    let cardPath = CGPath(roundedRect: cardRect, cornerWidth: 22, cornerHeight: 22, transform: nil)
    ctx.saveGState()
    ctx.setShadow(offset: CGSize(width: 0, height: -16), blur: 40, color: color(0x0F172A, 0.18))
    ctx.addPath(cardPath)
    ctx.setFillColor(color(0xFFFFFF))
    ctx.fillPath()
    ctx.restoreGState()
    ctx.addPath(cardPath)
    ctx.setStrokeColor(color(0xBFDBFE))
    ctx.setLineWidth(2)
    ctx.strokePath()

    drawText("正在智能解压文件...", rect: CGRect(x: 646, y: 770, width: 420, height: 32), font: .systemFont(ofSize: 22, weight: .bold), textColor: nsColor(0x0F172A))
    drawText("78%", rect: CGRect(x: 1120, y: 768, width: 154, height: 34), font: .monospacedSystemFont(ofSize: 24, weight: .bold), textColor: nsColor(0x2563EB), align: .right)

    // 进度条轨道与渐变填充
    let track = CGRect(x: 646, y: 822, width: 628, height: 18)
    ctx.addPath(CGPath(roundedRect: track, cornerWidth: 9, cornerHeight: 9, transform: nil))
    ctx.setFillColor(color(0xE2E8F0))
    ctx.fillPath()

    let fill = CGRect(x: 646, y: 822, width: 490, height: 18)
    ctx.saveGState()
    ctx.addPath(CGPath(roundedRect: fill, cornerWidth: 9, cornerHeight: 9, transform: nil))
    ctx.clip()
    drawLinear(ctx, [color(0x2563EB), color(0x38BDF8)], [0.0, 1.0], CGPoint(x: 646, y: 822), CGPoint(x: 1136, y: 822))
    ctx.restoreGState()

    drawText("正在释放：设计规范与图标源文件/主视觉海报_4K.png", rect: CGRect(x: 646, y: 862, width: 460, height: 28), font: .systemFont(ofSize: 16, weight: .regular), textColor: nsColor(0x64748B))
    drawText("1,002 / 1,284 项", rect: CGRect(x: 1110, y: 862, width: 164, height: 28), font: .monospacedSystemFont(ofSize: 16, weight: .medium), textColor: nsColor(0x475569), align: .right)
}
