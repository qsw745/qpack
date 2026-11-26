import Foundation
import AppKit
import CoreGraphics

// Windows 任务栏/桌面专用图标：透明底 + 满高 3D 磨砂玻璃折叠拉链盒（无白色底框，彻底解决在浅色任务栏显得偏矮的问题）
let size = 512
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
    exit(1)
}

ctx.interpolationQuality = .high
ctx.setAllowsAntialiasing(true)
ctx.setShouldAntialias(true)

// 坐标系转为左上角原点，并将原 1024 画布内 [158..870] 的 3D 盒体等比放大撑满 512x512（占 ~94% 高度）
ctx.translateBy(x: 0, y: H)
ctx.scaleBy(x: 1, y: -1)

// 原盒体中心 (512, 514)，总高 ~712px；缩放 0.68 -> 484px（占 512 画布的 94.5% 高度！）
let scale: CGFloat = 0.68
ctx.translateBy(x: W / 2.0, y: H / 2.0)
ctx.scaleBy(x: scale, y: scale)
ctx.translateBy(x: -512.0, y: -514.0)

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

let hex = roundedHexPath()

// 柔和贴边立体投影
ctx.saveGState()
ctx.setShadow(offset: CGSize(width: 0, height: -10), blur: 18, color: color(0x0F172A, 0.24))
ctx.addPath(hex)
ctx.setFillColor(color(0x2563EB, 1.0))
ctx.fillPath()
ctx.restoreGState()

// 三面磨砂玻璃绘制
ctx.saveGState()
ctx.addPath(hex)
ctx.clip()

// 左侧立面
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
ctx.addPath(polyPath([(234, 400), (480, 542), (480, 814), (234, 672)]))
ctx.setFillColor(color(0xFFFFFF, 0.14))
ctx.fillPath()
ctx.restoreGState()

// 右侧立面
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
ctx.addPath(polyPath([(544, 542), (790, 400), (790, 672), (544, 814)]))
ctx.setFillColor(color(0xFFFFFF, 0.18))
ctx.fillPath()
ctx.restoreGState()

// 顶面折叠盖板（在浅色任务栏上提高蓝调饱和度，确保顶边轮廓极其清晰）
let topFace = polyPath([(512, 158), (824, 338), (512, 518), (200, 338)])
ctx.saveGState()
ctx.addPath(topFace)
ctx.clip()
drawLinearGradient(
    [color(0xBAE6FD), color(0x7DD3FC), color(0x38BDF8)],
    [0.0, 0.50, 1.0],
    start: CGPoint(x: 240, y: 180),
    end: CGPoint(x: 760, y: 500)
)
ctx.addPath(polyPath([(512, 158), (668, 248), (356, 428), (200, 338)]))
ctx.setFillColor(color(0xFFFFFF, 0.30))
ctx.fillPath()
ctx.restoreGState()

// 棱线高光
ctx.saveGState()
ctx.setLineCap(.round)
ctx.setLineJoin(.round)
ctx.move(to: CGPoint(x: 204, y: 340))
ctx.addLine(to: CGPoint(x: 512, y: 518))
ctx.addLine(to: CGPoint(x: 820, y: 340))
ctx.setStrokeColor(color(0xFFFFFF, 0.88))
ctx.setLineWidth(7.0)
ctx.strokePath()

ctx.move(to: CGPoint(x: 512, y: 518))
ctx.addLine(to: CGPoint(x: 512, y: 868))
ctx.setStrokeColor(color(0xE0F2FE, 0.7))
ctx.setLineWidth(4.5)
ctx.strokePath()
ctx.restoreGState()
ctx.restoreGState()

// 外轮廓微蓝清晰描边
ctx.saveGState()
ctx.addPath(hex)
ctx.setStrokeColor(color(0x1D4ED8, 0.35))
ctx.setLineWidth(6.0)
ctx.strokePath()
ctx.restoreGState()

// 顶面拉链缝
ctx.saveGState()
ctx.setLineCap(.round)
ctx.move(to: CGPoint(x: 656, y: 256))
ctx.addLine(to: CGPoint(x: 368, y: 422))
ctx.setStrokeColor(color(0x0369A1, 0.32))
ctx.setLineWidth(14)
ctx.strokePath()
for i in 0..<10 {
    let t = CGFloat(i) / 9.0
    let cx = 640.0 * (1.0 - t) + 384.0 * t
    let cy = 265.0 * (1.0 - t) + 413.0 * t
    if abs(cx - 512) < 22 { continue }
    ctx.saveGState()
    ctx.translateBy(x: cx, y: cy)
    ctx.rotate(by: -CGFloat.pi / 6.0)
    ctx.addPath(CGPath(roundedRect: CGRect(x: -9, y: -5, width: 18, height: 10), cornerWidth: 5, cornerHeight: 5, transform: nil))
    ctx.setFillColor(color(0xFFFFFF, 0.95))
    ctx.fillPath()
    ctx.restoreGState()
}
ctx.restoreGState()

// 前棱拉链轨 + 拉链齿
let trackPath = CGPath(roundedRect: CGRect(x: 495, y: 514, width: 34, height: 260), cornerWidth: 17, cornerHeight: 17, transform: nil)
ctx.addPath(trackPath)
ctx.setFillColor(color(0x1E3A8A, 0.42))
ctx.fillPath()

for i in 0..<14 {
    let y = 536.0 + CGFloat(i) * 15.5
    let x: CGFloat = (i % 2 == 0) ? 494.0 : 513.0
    let tp = CGPath(roundedRect: CGRect(x: x, y: y, width: 18, height: 10), cornerWidth: 5, cornerHeight: 5, transform: nil)
    ctx.addPath(tp)
    ctx.setFillColor(color(0xF8FAFC, 0.95))
    ctx.fillPath()
}

// 铂金银拉链头（加粗增强小尺寸清晰度）
ctx.saveGState()
ctx.setShadow(offset: CGSize(width: 0, height: -8), blur: 14, color: color(0x0F172A, 0.34))
let sliderPath = CGPath(roundedRect: CGRect(x: 472, y: 468, width: 80, height: 74), cornerWidth: 22, cornerHeight: 22, transform: nil)
ctx.addPath(sliderPath)
ctx.setFillColor(color(0xFFFFFF))
ctx.fillPath()

let pullPath = CGMutablePath()
pullPath.move(to: CGPoint(x: 492, y: 534))
pullPath.addLine(to: CGPoint(x: 532, y: 534))
pullPath.addQuadCurve(to: CGPoint(x: 547, y: 552), control: CGPoint(x: 547, y: 534))
pullPath.addLine(to: CGPoint(x: 554, y: 650))
pullPath.addQuadCurve(to: CGPoint(x: 528, y: 678), control: CGPoint(x: 555, y: 678))
pullPath.addLine(to: CGPoint(x: 496, y: 678))
pullPath.addQuadCurve(to: CGPoint(x: 470, y: 650), control: CGPoint(x: 469, y: 678))
pullPath.addLine(to: CGPoint(x: 477, y: 552))
pullPath.addQuadCurve(to: CGPoint(x: 492, y: 534), control: CGPoint(x: 477, y: 534))
pullPath.closeSubpath()
ctx.addPath(pullPath)
ctx.setFillColor(color(0xF8FAFC))
ctx.fillPath()
ctx.restoreGState()

// 拉片镂空孔
let holePath = CGPath(roundedRect: CGRect(x: 494, y: 602, width: 36, height: 50), cornerWidth: 18, cornerHeight: 18, transform: nil)
ctx.saveGState()
ctx.addPath(holePath)
ctx.clip()
drawLinearGradient([color(0x1D4ED8), color(0x0EA5E9)], [0.0, 1.0], start: CGPoint(x: 494, y: 602), end: CGPoint(x: 530, y: 652))
ctx.restoreGState()

guard let outImg = ctx.makeImage() else { exit(1) }
let rep = NSBitmapImageRep(cgImage: outImg)
guard let png = rep.representation(using: .png, properties: [:]) else { exit(1) }
let outPath = "/tmp/qpack-win-full-512.png"
try png.write(to: URL(fileURLWithPath: outPath))
print("已生成 Windows 满高透明底 3D 图标: \(outPath)")
