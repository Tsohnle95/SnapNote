import AppKit

// Code-native artwork shared by the landing page, social preview, and release.
// Run from the repository root: swift scripts/generate-release-art.swift
let width = 1600.0
let height = 800.0
let canvas = NSImage(size: NSSize(width: width, height: height))
func color(_ hex: UInt32) -> NSColor {
    NSColor(srgbRed: Double((hex >> 16) & 255) / 255,
            green: Double((hex >> 8) & 255) / 255,
            blue: Double(hex & 255) / 255, alpha: 1)
}
func box(_ x: Double, _ y: Double, _ w: Double, _ h: Double,
         _ fill: UInt32, radius: Double = 0, border: UInt32? = nil) {
    let path = NSBezierPath(roundedRect: NSRect(x: x, y: y, width: w, height: h),
                            xRadius: radius, yRadius: radius)
    color(fill).setFill()
    path.fill()
    if let border { color(border).setStroke(); path.lineWidth = 1; path.stroke() }
}
func text(_ value: String, _ x: Double, _ y: Double, _ size: Double,
          _ hex: UInt32, font: String = "HelveticaNeue") {
    let attributes: [NSAttributedString.Key: Any] = [
        .font: NSFont(name: font, size: size) ?? NSFont.systemFont(ofSize: size),
        .foregroundColor: color(hex)
    ]
    (value as NSString).draw(at: NSPoint(x: x, y: y), withAttributes: attributes)
}
canvas.lockFocusFlipped(true)
box(0, 0, width, height, 0x191b1a)
box(72, 74, 52, 52, 0x30382a, radius: 14, border: 0x56614b)
let camera = NSBezierPath()
camera.move(to: NSPoint(x: 83, y: 91))
camera.line(to: NSPoint(x: 92, y: 91))
camera.line(to: NSPoint(x: 95, y: 86))
camera.line(to: NSPoint(x: 105, y: 86))
camera.line(to: NSPoint(x: 108, y: 91))
camera.line(to: NSPoint(x: 114, y: 91))
camera.line(to: NSPoint(x: 114, y: 113))
camera.line(to: NSPoint(x: 83, y: 113))
camera.close()
camera.lineWidth = 2
color(0xc4d2b7).setStroke(); camera.stroke()
let lens = NSBezierPath(ovalIn: NSRect(x: 92, y: 95, width: 13, height: 13))
lens.lineWidth = 2; lens.stroke()
text("SnapNote.", 139, 76, 34, 0xf1eee7)
text("A LITTLE SCREENSHOT COMPANION FOR MAC", 77, 215, 12, 0xa5aaa4)
text("A screenshot.", 70, 258, 88, 0xf1eee7)
text("With", 72, 355, 88, 0xf1eee7)
text("context.", 270, 349, 99, 0xefb699, font: "Georgia-Italic")
text("Capture what you see. Mark what matters.", 77, 495, 22, 0xb3b8af)
text("Add the thought that goes with it.", 77, 531, 22, 0xb3b8af)
box(77, 626, 205, 47, 0xefb699, radius: 7)
text("FREE FOR MAC  ↗", 100, 640, 15, 0x29231e)
text("Apple silicon  ·  macOS 12+", 77, 697, 13, 0xa5aaa4)

// A static illustrative capture, matching the website's product example.
box(872, 151, 646, 393, 0x20251e, border: 0x6b795d)
for (x, y) in [(869.0, 148.0), (1515.0, 148.0), (869.0, 541.0), (1515.0, 541.0)] {
    box(x, y, 6, 6, 0xefb699)
}
text("A LITTLE CONTEXT GOES A LONG WAY", 878, 119, 10, 0xb6c3a7)
box(883, 162, 624, 371, 0xefebe0, radius: 6)
box(883, 195, 624, 1, 0xd7d6cb)
text("○ ○ ○", 897, 171, 11, 0x8e9782)
text("Project notes", 1156, 173, 10, 0x7c8175)
text("THE NEXT LITTLE THING", 918, 222, 10, 0x7d856f)
text("Make room for", 916, 248, 43, 0x33392e, font: "Georgia")
text("the good ideas.", 916, 295, 43, 0x33392e, font: "Georgia")
text("A few thoughts on making something feel a little more human.", 918, 360, 13, 0x7b8172)
box(918, 401, 554, 1, 0xcfd2c3)
text("✓  A calmer place to start", 918, 417, 13, 0x555f4b)
text("□  Give this a little more space", 918, 454, 13, 0x555f4b)
text("□  Keep the important things close", 918, 491, 13, 0x929987)
let mark = NSBezierPath(roundedRect: NSRect(x: 908, y: 448, width: 462, height: 31), xRadius: 4, yRadius: 4)
color(0xc56852).setStroke(); mark.lineWidth = 2.5; mark.stroke()
let arrow = NSBezierPath()
arrow.move(to: NSPoint(x: 1407, y: 353))
arrow.curve(to: NSPoint(x: 1384, y: 439), controlPoint1: NSPoint(x: 1442, y: 386), controlPoint2: NSPoint(x: 1425, y: 418))
arrow.move(to: NSPoint(x: 1387, y: 416))
arrow.line(to: NSPoint(x: 1384, y: 439))
arrow.line(to: NSPoint(x: 1409, y: 435))
arrow.lineWidth = 2.5; arrow.stroke()
text("↖ Move      ✎ Pen      ↗ Arrow      □ Box      ↶   ↷", 1009, 562, 12, 0xa5aa9e)
box(872, 599, 646, 133, 0x272b26, radius: 12, border: 0x4d5547)
text("capture.png", 895, 615, 11, 0xa2ad99)
text("Let’s give this row a little more breathing room.", 895, 644, 17, 0xd8decf)
text("The rest is looking good.", 895, 672, 17, 0xd8decf)
box(1463, 685, 30, 26, 0x35432d, radius: 5, border: 0x4a5d3c)
text("✓", 1470, 689, 15, 0xc1daa8)
text("ILLUSTRATIVE CAPTURE", 1135, 755, 8, 0x8b9384)
canvas.unlockFocus()
guard let tiff = canvas.tiffRepresentation, let bitmap = NSBitmapImageRep(data: tiff),
      let png = bitmap.representation(using: .png, properties: [:]) else {
    fatalError("Could not render release artwork")
}
let path = "site/assets/snapnote-banner.png"
try png.write(to: URL(fileURLWithPath: path))
print("Rendered \(path)")

let button = NSImage(size: NSSize(width: 300, height: 64))
button.lockFocusFlipped(true)
box(0, 0, 300, 64, 0xefb699, radius: 9)
let download = NSBezierPath()
download.move(to: NSPoint(x: 27, y: 19))
download.line(to: NSPoint(x: 27, y: 39))
download.move(to: NSPoint(x: 20, y: 32))
download.line(to: NSPoint(x: 27, y: 39))
download.line(to: NSPoint(x: 34, y: 32))
download.move(to: NSPoint(x: 17, y: 41))
download.line(to: NSPoint(x: 17, y: 46))
download.line(to: NSPoint(x: 37, y: 46))
download.line(to: NSPoint(x: 37, y: 41))
color(0x29231e).setStroke(); download.lineWidth = 2; download.stroke()
text("Download for Mac", 54, 21, 22, 0x29231e, font: "HelveticaNeue-Medium")
text("↗", 267, 22, 20, 0x29231e)
button.unlockFocus()
guard let buttonTiff = button.tiffRepresentation,
      let buttonBitmap = NSBitmapImageRep(data: buttonTiff),
      let buttonPng = buttonBitmap.representation(using: .png, properties: [:]) else {
    fatalError("Could not render download button")
}
try buttonPng.write(to: URL(fileURLWithPath: "site/assets/download-mac.png"))
print("Rendered site/assets/download-mac.png")
