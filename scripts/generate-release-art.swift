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

// A concrete UI issue and an instruction that identifies the exact change.
box(872, 151, 646, 393, 0x20251e, border: 0x6b795d)
for (x, y) in [(869.0, 148.0), (1515.0, 148.0), (869.0, 541.0), (1515.0, 541.0)] {
    box(x, y, 6, 6, 0xefb699)
}
text("01  CAPTURE THE ISSUE. POINT TO THE DETAIL.", 878, 119, 10, 0xb6c3a7)
box(883, 162, 624, 371, 0xefebe0, radius: 6)
box(883, 195, 624, 1, 0xd7d6cb)
text("○ ○ ○", 897, 171, 11, 0x8e9782)
text("Your website · Account settings", 1099, 173, 10, 0x7c8175)
text("PROFILE", 918, 215, 10, 0x7d856f)
text("Account settings", 916, 238, 37, 0x33392e, font: "Georgia")
text("Keep your profile up to date.", 918, 288, 13, 0x7b8172)
text("Display name", 918, 321, 12, 0x555f4b)
box(918, 341, 554, 33, 0xf8f6ef, radius: 4, border: 0xd2d4c8)
text("Alex Morgan", 930, 350, 12, 0x727b67)
text("Email address", 918, 387, 12, 0x555f4b)
box(918, 407, 554, 33, 0xf8f6ef, radius: 4, border: 0xd2d4c8)
text("alex@example.com", 930, 416, 12, 0x727b67)
box(918, 471, 164, 36, 0xd4d7c8, radius: 4)
text("Save changes", 938, 480, 17, 0xa2aa92)
let mark = NSBezierPath(roundedRect: NSRect(x: 911, y: 464, width: 178, height: 50), xRadius: 4, yRadius: 4)
color(0xb6523d).setStroke(); mark.lineWidth = 2.5; mark.stroke()
let arrow = NSBezierPath()
arrow.move(to: NSPoint(x: 1190, y: 466))
arrow.curve(to: NSPoint(x: 1096, y: 488), controlPoint1: NSPoint(x: 1175, y: 504), controlPoint2: NSPoint(x: 1131, y: 505))
arrow.move(to: NSPoint(x: 1110, y: 479))
arrow.line(to: NSPoint(x: 1096, y: 488))
arrow.line(to: NSPoint(x: 1108, y: 501))
arrow.lineWidth = 2.5; arrow.stroke()
text("Low contrast", 1202, 477, 16, 0xa54331)
text("↖ Move      ✎ Pen      ↗ Arrow      □ Box      ↶   ↷", 1009, 553, 12, 0xa5aa9e)
text("02  TELL YOUR AGENT WHAT TO CHANGE.", 878, 581, 9, 0xb6c3a7)
box(872, 609, 646, 133, 0x272b26, radius: 12, border: 0x4d5547)
text("capture.png", 895, 625, 11, 0xa2ad99)
text("View this screenshot.", 895, 649, 12, 0xb6c3a7)
text("Make the boxed “Save changes” button dark green", 895, 671, 17, 0xd8decf)
text("with white text. Keep its size and position.", 895, 699, 17, 0xd8decf)
box(1463, 695, 30, 26, 0x35432d, radius: 5, border: 0x4a5d3c)
text("✓", 1470, 699, 15, 0xc1daa8)
text("ILLUSTRATIVE CAPTURE + AGENT INSTRUCTION", 1070, 761, 8, 0x8b9384)
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
