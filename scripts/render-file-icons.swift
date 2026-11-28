import Foundation
import AppKit
import CoreGraphics

struct FileTypeIconSpec {
    let key: String       // 输出文件名前缀，如 file-zip
    let label: String     // 底部徽章文字，如 ZIP / 7Z / TAR
    let accentTop: UInt32
    let accentBottom: UInt32
    let boxTop: UInt32
    let boxLeft: UInt32
    let boxRight: UInt32
}

let specs: [FileTypeIconSpec] = [
    FileTypeIconSpec(
        key: "file-zip",
        label: "ZIP",
        accentTop: 0x3B82F6,
        accentBottom: 0x1D4ED8,
        boxTop: 0x60A5FA,
        boxLeft: 0x2563EB,
        boxRight: 0x1E40AF
    ),
    FileTypeIconSpec(
        key: "file-7z",
        label: "7Z",
        accentTop: 0x10B981,
        accentBottom: 0x047857,
        boxTop: 0x34D399,
        boxLeft: 0x059669,
        boxRight: 0x065F46
    ),
    FileTypeIconSpec(
        key: "file-rar",
        label: "RAR",
        accentTop: 0xA855F7,
        accentBottom: 0x7E22CE,
        boxTop: 0xC084FC,
        boxLeft: 0x9333EA,
        boxRight: 0x6B21A8
    ),
    FileTypeIconSpec(
        key: "file-tar",
        label: "TAR",
        accentTop: 0xF59E0B,
        accentBottom: 0xB45309,
        boxTop: 0xFBBF24,
        boxLeft: 0xD97706,
        boxRight: 0x92400E
    ),
    FileTypeIconSpec(
        key: "file-gz",
        label: "GZ",
        accentTop: 0x0EA5E9,
        accentBottom: 0x0369A1,
        boxTop: 0x38BDF8,
        boxLeft: 0x0284C7,
        boxRight: 0x075985
    ),
    FileTypeIconSpec(
        key: "file-zst",
        label: "ZST",
        accentTop: 0x6366F1,
        accentBottom: 0x4338CA,
        boxTop: 0x818CF8,
        boxLeft: 0x4F46E5,
        boxRight: 0x3730A3
    ),
    FileTypeIconSpec(
        key: "file-archive",
        label: "ARC",
        accentTop: 0x64748B,
        accentBottom: 0x334155,
        boxTop: 0x94A3B8,
        boxLeft: 0x475569,
        boxRight: 0x1E293B
    ),
]

let cs = CGColorSpaceCreateDeviceRGB()

func cgColor(_ hex: UInt32, _ alpha: CGFloat = 1.0) -> CGColor {
    let r = CGFloat((hex >> 16) & 0xFF) / 255.0
    let g = CGFloat((hex >> 8) & 0xFF) / 255.0
    let b = CGFloat(hex & 0xFF) / 255.0
    return CGColor(colorSpace: cs, components: [r, g, b, alpha])!
}

func polyPath(_ pts: [(CGFloat, CGFloat)]) -> CGMutablePath {
    let p = CGMutablePath()
    guard let first = pts.first else { return p }
    p.move(to: CGPoint(x: first.0, y: first.1))
    for pt in pts.dropFirst() {
        p.addLine(to: CGPoint(x: pt.0, y: pt.1))
    }
    p.closeSubpath()
    return p
}

/// 渲染单张 1024x1024 的文档型压缩包文件图标（折角白纸 + QPack 立体盒 + 高辨识度底部格式徽章）
func renderFileIcon(_ spec: FileTypeIconSpec) -> CGImage {
    let size = 1024
    let ctx = CGContext(
        data: nil,
        width: size,
        height: size,
        bitsPerComponent: 8,
        bytesPerRow: 0,
        space: cs,
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    )!

    ctx.interpolationQuality = .high
    ctx.setAllowsAntialiasing(true)
    ctx.setShouldAntialias(true)

    // 左上角原点坐标系（绘制图形）
    ctx.saveGState()
    ctx.translateBy(x: 0, y: CGFloat(size))
    ctx.scaleBy(x: 1, y: -1)

    let left: CGFloat = 152
    let right: CGFloat = 872
    let top: CGFloat = 68
    let bottom: CGFloat = 956
    let fold: CGFloat = 196
    let radius: CGFloat = 64

    // 文档主体轮廓（右上角折角）
    let docPath = CGMutablePath()
    docPath.move(to: CGPoint(x: left + radius, y: top))
    docPath.addLine(to: CGPoint(x: right - fold, y: top))
    docPath.addLine(to: CGPoint(x: right, y: top + fold))
    docPath.addLine(to: CGPoint(x: right, y: bottom - radius))
    docPath.addArc(tangent1End: CGPoint(x: right, y: bottom), tangent2End: CGPoint(x: right - radius, y: bottom), radius: radius)
    docPath.addLine(to: CGPoint(x: left + radius, y: bottom))
    docPath.addArc(tangent1End: CGPoint(x: left, y: bottom), tangent2End: CGPoint(x: left, y: bottom - radius), radius: radius)
    docPath.addLine(to: CGPoint(x: left, y: top + radius))
    docPath.addArc(tangent1End: CGPoint(x: left, y: top), tangent2End: CGPoint(x: left + radius, y: top), radius: radius)
    docPath.closeSubpath()

    // 1. 柔和外投影
    ctx.saveGState()
    ctx.setShadow(offset: CGSize(width: 0, height: -18), blur: 36, color: cgColor(0x0F172A, 0.24))
    ctx.addPath(docPath)
    ctx.setFillColor(cgColor(0xFFFFFF, 1.0))
    ctx.fillPath()
    ctx.restoreGState()

    // 2. 纸张主体渐变填充
    ctx.saveGState()
    ctx.addPath(docPath)
    ctx.clip()
    let bgGrad = CGGradient(
        colorsSpace: cs,
        colors: [cgColor(0xFFFFFF), cgColor(0xF8FAFC), cgColor(0xEEF2F7)] as CFArray,
        locations: [0.0, 0.6, 1.0]
    )!
    ctx.drawLinearGradient(bgGrad, start: CGPoint(x: left, y: top), end: CGPoint(x: right, y: bottom), options: [])

    // 纸张顶部拉链虚线纹理（点缀压缩包特征）
    let zipX: CGFloat = 310
    for i in 0..<7 {
        let y = top + 36 + CGFloat(i) * 34
        let toothX = (i % 2 == 0) ? zipX - 20 : zipX + 2
        let toothRect = CGRect(x: toothX, y: y, width: 22, height: 18)
        let toothPath = CGPath(roundedRect: toothRect, cornerWidth: 6, cornerHeight: 6, transform: nil)
        ctx.addPath(toothPath)
        ctx.setFillColor(cgColor(0xCBD5E1, 0.85))
        ctx.fillPath()
    }

    // 3. 底部高对比度彩色格式徽章区（完整铺满纸张底部，小尺寸下也极易辨认）
    let bandTop: CGFloat = 640
    let bandRect = CGRect(x: left, y: bandTop, width: right - left, height: bottom - bandTop)
    let bandGrad = CGGradient(
        colorsSpace: cs,
        colors: [cgColor(spec.accentTop), cgColor(spec.accentBottom)] as CFArray,
        locations: [0.0, 1.0]
    )!
    ctx.saveGState()
    ctx.clip(to: bandRect)
    ctx.drawLinearGradient(bandGrad, start: CGPoint(x: left, y: bandTop), end: CGPoint(x: left, y: bottom), options: [])
    // 顶部细高光边
    ctx.setFillColor(cgColor(0xFFFFFF, 0.25))
    ctx.fill(CGRect(x: left, y: bandTop, width: right - left, height: 6))
    ctx.restoreGState()

    // 4. 中央 QPack 标志性立体六角盒图形
    let cx: CGFloat = 512
    let cy: CGFloat = 415
    let r: CGFloat = 158
    let dx = r * 0.866
    let dy = r * 0.5

    let topFace = polyPath([
        (cx, cy - r),
        (cx + dx, cy - dy),
        (cx, cy),
        (cx - dx, cy - dy),
    ])
    let leftFace = polyPath([
        (cx - dx, cy - dy),
        (cx, cy),
        (cx, cy + r),
        (cx - dx, cy + dy),
    ])
    let rightFace = polyPath([
        (cx, cy),
        (cx + dx, cy - dy),
        (cx + dx, cy + dy),
        (cx, cy + r),
    ])

    // 盒体微投影
    ctx.saveGState()
    ctx.setShadow(offset: CGSize(width: 0, height: -10), blur: 22, color: cgColor(spec.accentBottom, 0.28))
    ctx.addPath(leftFace)
    ctx.addPath(rightFace)
    ctx.addPath(topFace)
    ctx.setFillColor(cgColor(spec.boxLeft))
    ctx.fillPath()
    ctx.restoreGState()

    ctx.addPath(topFace)
    ctx.setFillColor(cgColor(spec.boxTop))
    ctx.fillPath()

    ctx.addPath(leftFace)
    ctx.setFillColor(cgColor(spec.boxLeft))
    ctx.fillPath()

    ctx.addPath(rightFace)
    ctx.setFillColor(cgColor(spec.boxRight))
    ctx.fillPath()

    // 盒面白色封箱带与高光棱线
    let tapePath = polyPath([
        (cx - dx * 0.34, cy - dy * 1.34),
        (cx - dx * 0.08, cy - dy * 1.60),
        (cx + dx * 0.52, cy - dy * 0.42),
        (cx + dx * 0.26, cy - dy * 0.16),
    ])
    ctx.addPath(tapePath)
    ctx.setFillColor(cgColor(0xFFFFFF, 0.35))
    ctx.fillPath()

    ctx.setStrokeColor(cgColor(0xFFFFFF, 0.45))
    ctx.setLineWidth(7)
    ctx.setLineCap(.round)
    ctx.move(to: CGPoint(x: cx - dx, y: cy - dy))
    ctx.addLine(to: CGPoint(x: cx, y: cy))
    ctx.addLine(to: CGPoint(x: cx + dx, y: cy - dy))
    ctx.move(to: CGPoint(x: cx, y: cy))
    ctx.addLine(to: CGPoint(x: cx, y: cy + r))
    ctx.strokePath()

    ctx.restoreGState() // 退出 docPath clip

    // 5. 右上角折页三角形 + 阴影
    let foldPath = CGMutablePath()
    foldPath.move(to: CGPoint(x: right - fold, y: top))
    foldPath.addLine(to: CGPoint(x: right - fold, y: top + fold - 28))
    foldPath.addArc(
        tangent1End: CGPoint(x: right - fold, y: top + fold),
        tangent2End: CGPoint(x: right - fold + 28, y: top + fold),
        radius: 28
    )
    foldPath.addLine(to: CGPoint(x: right, y: top + fold))
    foldPath.closeSubpath()

    ctx.saveGState()
    ctx.setShadow(offset: CGSize(width: -6, height: -8), blur: 16, color: cgColor(0x0F172A, 0.16))
    ctx.addPath(foldPath)
    ctx.setFillColor(cgColor(0xE2E8F0, 1.0))
    ctx.fillPath()
    ctx.restoreGState()

    // 6. 纸张细腻外描边（浅色/深色壁纸下均清晰有轮廓）
    ctx.addPath(docPath)
    ctx.setStrokeColor(cgColor(0xCBD5E1, 0.9))
    ctx.setLineWidth(12)
    ctx.strokePath()

    ctx.restoreGState() // 恢复正常 macOS 左下角原点坐标系，用于 NSString 精准居中绘制

    // 7. 绘制底部粗体格式标签（ZIP / 7Z / RAR / TAR / GZ / ZST / ARC）
    let nsCtx = NSGraphicsContext(cgContext: ctx, flipped: false)
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = nsCtx

    let fontSize: CGFloat = spec.label.count <= 2 ? 220 : 192
    let font = NSFont.systemFont(ofSize: fontSize, weight: .heavy)
    let para = NSMutableParagraphStyle()
    para.alignment = .center

    let shadow = NSShadow()
    shadow.shadowColor = NSColor(white: 0.0, alpha: 0.25)
    shadow.shadowOffset = NSSize(width: 0, height: -5)
    shadow.shadowBlurRadius = 10

    let attrs: [NSAttributedString.Key: Any] = [
        .font: font,
        .foregroundColor: NSColor.white,
        .paragraphStyle: para,
        .shadow: shadow,
        .kern: 6.0,
    ]
    let str = NSAttributedString(string: spec.label, attributes: attrs)
    let textSize = str.size()

    // 左下角坐标系下，band 在 y = [68, 384]（中心 y = 226）
    let bandCenterY: CGFloat = CGFloat(size) - (bandTop + bottom) / 2.0
    let drawRect = NSRect(
        x: left,
        y: bandCenterY - textSize.height / 2.0 - 4,
        width: right - left,
        height: textSize.height + 8
    )
    str.draw(in: drawRect)
    NSGraphicsContext.restoreGraphicsState()

    return ctx.makeImage()!
}

func pngData(from image: CGImage, size: Int) -> Data {
    let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil,
        pixelsWide: size,
        pixelsHigh: size,
        bitsPerSample: 8,
        samplesPerPixel: 4,
        hasAlpha: true,
        isPlanar: false,
        colorSpaceName: .deviceRGB,
        bytesPerRow: 0,
        bitsPerPixel: 0
    )!
    rep.size = NSSize(width: size, height: size)

    NSGraphicsContext.saveGraphicsState()
    let gctx = NSGraphicsContext(bitmapImageRep: rep)!
    gctx.imageInterpolation = .high
    NSGraphicsContext.current = gctx
    gctx.cgContext.draw(image, in: CGRect(x: 0, y: 0, width: size, height: size))
    NSGraphicsContext.restoreGraphicsState()

    return rep.representation(using: .png, properties: [:])!
}

/// 打包多尺寸 Windows ICO（PNG-in-ICO，支持 16~256px，Vista+ 完美兼容）
func writeIco(from image: CGImage, to url: URL) throws {
    let sizes = [16, 24, 32, 48, 64, 128, 256]
    let pngs = sizes.map { pngData(from: image, size: $0) }

    var data = Data()
    // ICONDIR header
    data.append(contentsOf: [0, 0, 1, 0])
    var count = UInt16(sizes.count).littleEndian
    data.append(Data(bytes: &count, count: 2))

    var offset = UInt32(6 + 16 * sizes.count)
    for (idx, sz) in sizes.enumerated() {
        let dim: UInt8 = sz >= 256 ? 0 : UInt8(sz)
        data.append(dim)          // width
        data.append(dim)          // height
        data.append(0)            // palette
        data.append(0)            // reserved
        data.append(contentsOf: [1, 0])   // color planes
        data.append(contentsOf: [32, 0])  // bpp
        var bytesInRes = UInt32(pngs[idx].count).littleEndian
        data.append(Data(bytes: &bytesInRes, count: 4))
        var imgOffset = offset.littleEndian
        data.append(Data(bytes: &imgOffset, count: 4))
        offset += UInt32(pngs[idx].count)
    }

    for png in pngs {
        data.append(png)
    }
    try data.write(to: url)
}

/// 使用 macOS 原生 iconutil 生成全尺寸 Retina .icns
func writeIcns(from image: CGImage, to url: URL, tempRoot: URL, key: String) throws {
    let iconsetDir = tempRoot.appendingPathComponent("\(key).iconset")
    let fm = FileManager.default
    try? fm.removeItem(at: iconsetDir)
    try fm.createDirectory(at: iconsetDir, withIntermediateDirectories: true)

    let entries: [(String, Int)] = [
        ("icon_16x16.png", 16),
        ("icon_16x16@2x.png", 32),
        ("icon_32x32.png", 32),
        ("icon_32x32@2x.png", 64),
        ("icon_128x128.png", 128),
        ("icon_128x128@2x.png", 256),
        ("icon_256x256.png", 256),
        ("icon_256x256@2x.png", 512),
        ("icon_512x512.png", 512),
        ("icon_512x512@2x.png", 1024),
    ]

    for (name, sz) in entries {
        let d = pngData(from: image, size: sz)
        try d.write(to: iconsetDir.appendingPathComponent(name))
    }

    let proc = Process()
    proc.executableURL = URL(fileURLWithPath: "/usr/bin/iconutil")
    proc.arguments = ["-c", "icns", iconsetDir.path, "-o", url.path]
    try proc.run()
    proc.waitUntilExit()
    if proc.terminationStatus != 0 {
        fputs("iconutil 生成 \(url.lastPathComponent) 失败\n", stderr)
        exit(1)
    }
    try? fm.removeItem(at: iconsetDir)
}

let outDir = URL(fileURLWithPath: "src-tauri/icons/file-types")
let tempDir = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent("qpack-file-icons-\(UUID().uuidString)")
let fm = FileManager.default
try fm.createDirectory(at: outDir, withIntermediateDirectories: true)
try fm.createDirectory(at: tempDir, withIntermediateDirectories: true)

for spec in specs {
    let img = renderFileIcon(spec)
    let icoUrl = outDir.appendingPathComponent("\(spec.key).ico")
    let icnsUrl = outDir.appendingPathComponent("\(spec.key).icns")
    try writeIco(from: img, to: icoUrl)
    try writeIcns(from: img, to: icnsUrl, tempRoot: tempDir, key: spec.key)
    print("✓ 已生成 \(spec.key).ico & \(spec.key).icns (\(spec.label))")
}

// 额外输出一张预览图用于人工检查
let previewImg = renderFileIcon(specs[1]) // 7Z 示例
try pngData(from: previewImg, size: 256).write(to: tempDir.appendingPathComponent("preview-7z.png"))
print("PREVIEW_PATH=\(tempDir.appendingPathComponent("preview-7z.png").path)")
