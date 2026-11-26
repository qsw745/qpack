import Foundation
import AppKit
import CoreGraphics

let size = 1024
let W = CGFloat(size)
let H = CGFloat(size)
let cs = CGColorSpaceCreateDeviceRGB()

guard let ctx = CGContext(
    data: nil,
    width: size,
    height: size,
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

// 转为左上角原点坐标系（与 SVG / 设计稿一致）
ctx.translateBy(x: 0, y: H)
ctx.scaleBy(x: 1, y: -1)

func color(_ hex: UInt32, _ alpha: CGFloat = 1.0) -> CGColor {
    let r = CGFloat((hex >> 16) & 0xFF) / 255.0
    let g = CGFloat((hex >> 8) & 0xFF) / 255.0
    let b = CGFloat(hex & 0xFF) / 255.0
    return CGColor(colorSpace: cs, components: [r, g, b, alpha])!
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

func roundedHexPath() -> CGMutablePath {
    let p = CGMutablePath()
    p.move(to: CGPoint(x: 482, y: 176))
    p.addQuadCurve(to: CGPoint(x: 542, y: 176), control: CGPoint(x: 512, y: 158))
    p.addLine(to: CGPoint(x: 792, y: 320))
    p.addQuadCurve(to: CGPoint(x: 822, y: 372), control: CGPoint(x: 822, y: 338))
    p.addLine(to: CGPoint(x: 822, y: 656))
    p.addQuadCurve(to: CGPoint(x: 792, y: 708), control: CGPoint(x: 822, y: 690))
    p.addLine(to: CGPoint(x: 542, y: 852))
    p.addQuadCurve(to: CGPoint(x: 482, y: 852), control: CGPoint(x: 512, y: 870))
    p.addLine(to: CGPoint(x: 232, y: 708))
    p.addQuadCurve(to: CGPoint(x: 202, y: 656), control: CGPoint(x: 202, y: 690))
    p.addLine(to: CGPoint(x: 202, y: 372))
    p.addQuadCurve(to: CGPoint(x: 232, y: 320), control: CGPoint(x: 202, y: 338))
    p.closeSubpath()
    return p
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

// 1. 珍珠白微光背景
ctx.saveGState()
drawLinearGradient(
    [color(0xFFFFFF), color(0xF5F8FE), color(0xE6EFFB)],
    [0.0, 0.52, 1.0],
    start: CGPoint(x: 0, y: 0),
    end: CGPoint(x: W, y: H)
)
drawRadialGradient(
    [color(0xFFFFFF, 0.95), color(0xEFF6FF, 0.35), color(0xDBEAFE, 0.0)],
    [0.0, 0.55, 1.0],
    center: CGPoint(x: 340, y: 210),
    radius: 680
)
// 底部蓝调玻璃折射柔光投影
ctx.saveGState()
ctx.translateBy(x: 512, y: 810)
ctx.scaleBy(x: 1.0, y: 0.34)
drawRadialGradient(
    [color(0x2563EB, 0.24), color(0x3B82F6, 0.10), color(0x60A5FA, 0.0)],
    [0.0, 0.55, 1.0],
    center: .zero,
    radius: 340
)
ctx.restoreGState()
ctx.restoreGState()

let hex = roundedHexPath()

// 2. 盒体多层柔和落地阴影
ctx.saveGState()
ctx.setShadow(offset: CGSize(width: 0, height: -28), blur: 48, color: color(0x1E3A8A, 0.22))
ctx.addPath(hex)
ctx.setFillColor(color(0xFFFFFF, 1.0))
ctx.fillPath()
ctx.restoreGState()

ctx.saveGState()
ctx.setShadow(offset: CGSize(width: 0, height: -12), blur: 20, color: color(0x0F172A, 0.14))
ctx.addPath(hex)
ctx.setFillColor(color(0xFFFFFF, 1.0))
ctx.fillPath()
ctx.restoreGState()

// 3. 3D 磨砂玻璃折叠盒三面绘制（裁剪在圆角六边形内）
ctx.saveGState()
ctx.addPath(hex)
ctx.clip()

// 3a. 左侧立面（钴蓝磨砂玻璃）
let leftFace = polyPath([(200, 340), (512, 520), (512, 872), (200, 692)])
ctx.saveGState()
ctx.addPath(leftFace)
ctx.clip()
drawLinearGradient(
    [color(0x4F8EF7), color(0x2563EB), color(0x1D4ED8)],
    [0.0, 0.52, 1.0],
    start: CGPoint(x: 200, y: 340),
    end: CGPoint(x: 512, y: 860)
)
// 左侧内部半透明磨砂玻璃面板
let leftInner = polyPath([(234, 400), (480, 542), (480, 814), (234, 672)])
ctx.addPath(leftInner)
ctx.setFillColor(color(0xFFFFFF, 0.14))
ctx.fillPath()
// 左侧斜向玻璃高光带
let leftStreak = polyPath([(200, 340), (390, 450), (295, 745), (200, 692)])
ctx.saveGState()
ctx.addPath(leftStreak)
ctx.clip()
drawLinearGradient(
    [color(0xFFFFFF, 0.28), color(0xFFFFFF, 0.0)],
    [0.0, 1.0],
    start: CGPoint(x: 210, y: 350),
    end: CGPoint(x: 370, y: 690)
)
ctx.restoreGState()
ctx.restoreGState()

// 3b. 右侧立面（青蓝磨砂玻璃）
let rightFace = polyPath([(512, 520), (824, 340), (824, 692), (512, 872)])
ctx.saveGState()
ctx.addPath(rightFace)
ctx.clip()
drawLinearGradient(
    [color(0x56CCF2), color(0x0EA5E9), color(0x0284C7)],
    [0.0, 0.50, 1.0],
    start: CGPoint(x: 512, y: 340),
    end: CGPoint(x: 824, y: 860)
)
// 右侧内部半透明磨砂玻璃面板
let rightInner = polyPath([(544, 542), (790, 400), (790, 672), (544, 814)])
ctx.addPath(rightInner)
ctx.setFillColor(color(0xFFFFFF, 0.18))
ctx.fillPath()
// 右侧斜向玻璃高光带
let rightStreak = polyPath([(620, 460), (824, 340), (824, 570), (665, 780)])
ctx.saveGState()
ctx.addPath(rightStreak)
ctx.clip()
drawLinearGradient(
    [color(0xFFFFFF, 0.30), color(0xFFFFFF, 0.0)],
    [0.0, 1.0],
    start: CGPoint(x: 780, y: 350),
    end: CGPoint(x: 650, y: 740)
)
ctx.restoreGState()
ctx.restoreGState()

// 3c. 顶面折叠盖板（晶莹冰蓝磨砂玻璃）
let topFace = polyPath([(512, 158), (824, 338), (512, 518), (200, 338)])
ctx.saveGState()
ctx.addPath(topFace)
ctx.clip()
drawLinearGradient(
    [color(0xF8FCFF), color(0xCBEAFF), color(0x7CC4FA)],
    [0.0, 0.52, 1.0],
    start: CGPoint(x: 240, y: 180),
    end: CGPoint(x: 760, y: 500)
)
// 左上折叠盖板微提亮
let topFlapLeft = polyPath([(512, 158), (668, 248), (356, 428), (200, 338)])
ctx.addPath(topFlapLeft)
ctx.setFillColor(color(0xFFFFFF, 0.32))
ctx.fillPath()
// 顶面镜面高光斜条
let topHighlight = polyPath([(310, 275), (512, 158), (628, 225), (420, 346)])
ctx.saveGState()
ctx.addPath(topHighlight)
ctx.clip()
drawLinearGradient(
    [color(0xFFFFFF, 0.65), color(0xFFFFFF, 0.0)],
    [0.0, 1.0],
    start: CGPoint(x: 360, y: 200),
    end: CGPoint(x: 560, y: 320)
)
ctx.restoreGState()
ctx.restoreGState()

// 3d. 顶面与前棱玻璃厚度高光棱线
ctx.saveGState()
ctx.setLineCap(.round)
ctx.setLineJoin(.round)
ctx.move(to: CGPoint(x: 204, y: 340))
ctx.addLine(to: CGPoint(x: 512, y: 518))
ctx.addLine(to: CGPoint(x: 820, y: 340))
ctx.setStrokeColor(color(0xFFFFFF, 0.85))
ctx.setLineWidth(5.5)
ctx.strokePath()

ctx.move(to: CGPoint(x: 512, y: 518))
ctx.addLine(to: CGPoint(x: 512, y: 868))
ctx.setStrokeColor(color(0xE0F2FE, 0.65))
ctx.setLineWidth(3.5)
ctx.strokePath()
ctx.restoreGState()

ctx.restoreGState() // end hex clip

// 盒体外轮廓玻璃描边
ctx.saveGState()
ctx.addPath(hex)
ctx.setStrokeColor(color(0xFFFFFF, 0.78))
ctx.setLineWidth(4.5)
ctx.strokePath()
ctx.restoreGState()

// 4. 顶面折叠拉链缝与微拉链齿
ctx.saveGState()
ctx.setLineCap(.round)
ctx.move(to: CGPoint(x: 656, y: 256))
ctx.addLine(to: CGPoint(x: 368, y: 422))
ctx.setStrokeColor(color(0x0369A1, 0.24))
ctx.setLineWidth(11)
ctx.strokePath()

for i in 0..<12 {
    let t = CGFloat(i) / 11.0
    let cx = 640.0 * (1.0 - t) + 384.0 * t
    let cy = 265.0 * (1.0 - t) + 413.0 * t
    if abs(cx - 512) < 18 { continue }
    ctx.saveGState()
    ctx.translateBy(x: cx, y: cy)
    ctx.rotate(by: -CGFloat.pi / 6.0)
    let tooth = CGPath(roundedRect: CGRect(x: -8, y: -4.5, width: 16, height: 9), cornerWidth: 4.5, cornerHeight: 4.5, transform: nil)
    ctx.addPath(tooth)
    ctx.setFillColor(color(0xF8FAFC, 0.92))
    ctx.fillPath()
    ctx.restoreGState()
}
ctx.restoreGState()

// 5. 前中棱垂直拉链轨道与金属拉链齿
let trackRect = CGRect(x: 497, y: 514, width: 30, height: 260)
let trackPath = CGPath(roundedRect: trackRect, cornerWidth: 15, cornerHeight: 15, transform: nil)
ctx.saveGState()
ctx.addPath(trackPath)
ctx.setFillColor(color(0x1E3A8A, 0.36))
ctx.fillPath()
ctx.restoreGState()

for i in 0..<18 {
    let y = 534.0 + CGFloat(i) * 12.2
    let x: CGFloat = (i % 2 == 0) ? 496.0 : 512.0
    let toothRect = CGRect(x: x, y: y, width: 16, height: 9)
    let toothPath = CGPath(roundedRect: toothRect, cornerWidth: 4.5, cornerHeight: 4.5, transform: nil)
    ctx.saveGState()
    ctx.addPath(toothPath)
    ctx.clip()
    drawLinearGradient(
        [color(0xFFFFFF), color(0xCBD5E1)],
        [0.0, 1.0],
        start: CGPoint(x: x, y: y),
        end: CGPoint(x: x + 16, y: y + 9)
    )
    ctx.restoreGState()
}

// 6. 铂金银精密拉链头（Slider + Pull Tab）
func drawMetallicRoundedRect(_ rect: CGRect, radius: CGFloat, strokeWidth: CGFloat = 3.0) {
    let p = CGPath(roundedRect: rect, cornerWidth: radius, cornerHeight: radius, transform: nil)
    ctx.saveGState()
    ctx.addPath(p)
    ctx.clip()
    drawLinearGradient(
        [color(0xFFFFFF), color(0xF1F5F9), color(0xCBD5E1), color(0x94A3B8)],
        [0.0, 0.35, 0.72, 1.0],
        start: CGPoint(x: rect.minX, y: rect.minY),
        end: CGPoint(x: rect.maxX, y: rect.maxY)
    )
    ctx.restoreGState()
    ctx.saveGState()
    ctx.addPath(p)
    ctx.setStrokeColor(color(0xFFFFFF, 0.9))
    ctx.setLineWidth(strokeWidth)
    ctx.strokePath()
    ctx.restoreGState()
}

// 拉链头整体投影
ctx.saveGState()
ctx.setShadow(offset: CGSize(width: 0, height: -10), blur: 18, color: color(0x0F172A, 0.30))

// 滑块主体
let sliderRect = CGRect(x: 478, y: 474, width: 68, height: 64)
ctx.addPath(CGPath(roundedRect: sliderRect, cornerWidth: 19, cornerHeight: 19, transform: nil))
ctx.setFillColor(color(0xF8FAFC))
ctx.fillPath()

// 拉片主体（圆角微梯形）
let pullPath = CGMutablePath()
pullPath.move(to: CGPoint(x: 495, y: 534))
pullPath.addLine(to: CGPoint(x: 529, y: 534))
pullPath.addQuadCurve(to: CGPoint(x: 541, y: 548), control: CGPoint(x: 541, y: 534))
pullPath.addLine(to: CGPoint(x: 548, y: 638))
pullPath.addQuadCurve(to: CGPoint(x: 526, y: 662), control: CGPoint(x: 549, y: 662))
pullPath.addLine(to: CGPoint(x: 498, y: 662))
pullPath.addQuadCurve(to: CGPoint(x: 476, y: 638), control: CGPoint(x: 475, y: 662))
pullPath.addLine(to: CGPoint(x: 483, y: 548))
pullPath.addQuadCurve(to: CGPoint(x: 495, y: 534), control: CGPoint(x: 483, y: 534))
pullPath.closeSubpath()

ctx.addPath(pullPath)
ctx.setFillColor(color(0xF8FAFC))
ctx.fillPath()
ctx.restoreGState()

// 金属材质覆盖绘制
drawMetallicRoundedRect(sliderRect, radius: 19, strokeWidth: 3.0)
// 滑块顶盖凹槽
let capRect = CGRect(x: 492, y: 487, width: 40, height: 22)
ctx.saveGState()
ctx.addPath(CGPath(roundedRect: capRect, cornerWidth: 9, cornerHeight: 9, transform: nil))
ctx.setFillColor(color(0xE2E8F0))
ctx.fillPath()
ctx.setStrokeColor(color(0xCBD5E1))
ctx.setLineWidth(1.5)
ctx.strokePath()
ctx.restoreGState()

// 铰链扣环
drawMetallicRoundedRect(CGRect(x: 499, y: 516, width: 26, height: 30), radius: 13, strokeWidth: 2.0)

// 拉片金属渐变
ctx.saveGState()
ctx.addPath(pullPath)
ctx.clip()
drawLinearGradient(
    [color(0xFFFFFF), color(0xF1F5F9), color(0xCBD5E1), color(0x94A3B8)],
    [0.0, 0.30, 0.72, 1.0],
    start: CGPoint(x: 476, y: 534),
    end: CGPoint(x: 548, y: 662)
)
ctx.restoreGState()

ctx.saveGState()
ctx.addPath(pullPath)
ctx.setStrokeColor(color(0xFFFFFF, 0.92))
ctx.setLineWidth(3.0)
ctx.strokePath()
ctx.restoreGState()

// 拉片镂空孔（透出深蓝背景色）
let holeRect = CGRect(x: 496, y: 594, width: 32, height: 46)
let holePath = CGPath(roundedRect: holeRect, cornerWidth: 16, cornerHeight: 16, transform: nil)
ctx.saveGState()
ctx.addPath(holePath)
ctx.clip()
drawLinearGradient(
    [color(0x1D4ED8), color(0x0EA5E9)],
    [0.0, 1.0],
    start: CGPoint(x: 496, y: 594),
    end: CGPoint(x: 528, y: 640)
)
ctx.restoreGState()
ctx.saveGState()
ctx.addPath(holePath)
ctx.setStrokeColor(color(0x94A3B8, 0.9))
ctx.setLineWidth(2.5)
ctx.strokePath()
ctx.restoreGState()

// 拉片顶部微高光
ctx.saveGState()
ctx.setFillColor(color(0xFFFFFF, 0.9))
ctx.fillEllipse(in: CGRect(x: 500, y: 558, width: 24, height: 10))
ctx.restoreGState()

// 输出 PNG
guard let img = ctx.makeImage() else {
    fputs("生成图片失败\n", stderr)
    exit(1)
}
let rep = NSBitmapImageRep(cgImage: img)
guard let data = rep.representation(using: .png, properties: [:]) else {
    fputs("编码 PNG 失败\n", stderr)
    exit(1)
}
let outPath = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "assets/icon-source.png"
try data.write(to: URL(fileURLWithPath: outPath))
print("已生成高清图标源图: \(outPath)")
