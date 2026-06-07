#!/usr/bin/swift
// Generates the macOS AppIcon PNG set (placeholder: "VCT" on a gradient).
// Usage: swift scripts/generate-app-icon.swift [path/to/AppIcon.appiconset]
import AppKit
import Foundation

let defaultOut = "VibeCodeTrackerApp/Resources/Assets.xcassets/AppIcon.appiconset"
let outDir = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : defaultOut

let specs: [(px: Int, idiom: String, size: String, scale: String, file: String)] = [
    (16, "mac", "16x16", "1x", "icon_16.png"),
    (32, "mac", "16x16", "2x", "icon_16@2x.png"),
    (32, "mac", "32x32", "1x", "icon_32.png"),
    (64, "mac", "32x32", "2x", "icon_32@2x.png"),
    (128, "mac", "128x128", "1x", "icon_128.png"),
    (256, "mac", "128x128", "2x", "icon_128@2x.png"),
    (256, "mac", "256x256", "1x", "icon_256.png"),
    (512, "mac", "256x256", "2x", "icon_256@2x.png"),
    (512, "mac", "512x512", "1x", "icon_512.png"),
    (1024, "mac", "512x512", "2x", "icon_512@2x.png"),
]

func render(_ size: Int) -> Data? {
    let s = CGFloat(size)
    let image = NSImage(size: NSSize(width: s, height: s))
    image.lockFocus()
    guard let ctx = NSGraphicsContext.current?.cgContext else { image.unlockFocus(); return nil }

    let inset = s * 0.07
    let rect = CGRect(x: inset, y: inset, width: s - 2 * inset, height: s - 2 * inset)
    let radius = s * 0.225
    let clip = NSBezierPath(roundedRect: rect, xRadius: radius, yRadius: radius)
    clip.addClip()

    let colors = [
        NSColor(srgbRed: 0.36, green: 0.30, blue: 0.92, alpha: 1).cgColor,
        NSColor(srgbRed: 0.58, green: 0.24, blue: 0.86, alpha: 1).cgColor,
    ] as CFArray
    if let grad = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(), colors: colors, locations: [0, 1]) {
        ctx.drawLinearGradient(grad, start: CGPoint(x: 0, y: s), end: CGPoint(x: s, y: 0), options: [])
    }

    let text = "VCT" as NSString
    let style = NSMutableParagraphStyle()
    style.alignment = .center
    let attrs: [NSAttributedString.Key: Any] = [
        .font: NSFont.systemFont(ofSize: s * 0.28, weight: .heavy),
        .foregroundColor: NSColor.white,
        .paragraphStyle: style,
    ]
    let textSize = text.size(withAttributes: attrs)
    text.draw(at: CGPoint(x: (s - textSize.width) / 2, y: (s - textSize.height) / 2), withAttributes: attrs)

    image.unlockFocus()
    guard let tiff = image.tiffRepresentation, let rep = NSBitmapImageRep(data: tiff) else { return nil }
    return rep.representation(using: .png, properties: [:])
}

let fm = FileManager.default
try? fm.createDirectory(atPath: outDir, withIntermediateDirectories: true)

for spec in specs {
    guard let data = render(spec.px) else { fatalError("render failed at \(spec.px)") }
    let path = "\(outDir)/\(spec.file)"
    try data.write(to: URL(fileURLWithPath: path))
    print("wrote \(path) (\(spec.px)px)")
}

let images = specs.map { spec in
    """
        {
          "idiom" : "\(spec.idiom)",
          "size" : "\(spec.size)",
          "scale" : "\(spec.scale)",
          "filename" : "\(spec.file)"
        }
    """
}.joined(separator: ",\n")

let contents = """
{
  "images" : [
\(images)
  ],
  "info" : {
    "author" : "xcode",
    "version" : 1
  }
}
"""
try contents.write(to: URL(fileURLWithPath: "\(outDir)/Contents.json"), atomically: true, encoding: .utf8)
print("wrote \(outDir)/Contents.json")
