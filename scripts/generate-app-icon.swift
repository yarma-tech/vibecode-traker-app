#!/usr/bin/swift
// Generates the macOS AppIcon PNG set: the "VE" logo mark (white knockout) on
// the brand purple gradient, in a rounded macOS tile.
// Usage: swift scripts/generate-app-icon.swift [path/to/AppIcon.appiconset] [path/to/mark.png]
import AppKit
import Foundation

let defaultOut = "VibeCodeTrackerApp/Resources/Assets.xcassets/AppIcon.appiconset"
let outDir = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : defaultOut
let defaultMark = "VibeCodeTrackerApp/Resources/Assets.xcassets/LogoMark.imageset/mark@2x.png"
let markPath = CommandLine.arguments.count > 2 ? CommandLine.arguments[2] : defaultMark

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

// MARK: - Load the logo mark, trim transparent padding, build a white silhouette

/// Bounding box of the non-transparent pixels in `cg`.
func alphaBounds(_ cg: CGImage) -> CGRect {
    let w = cg.width, h = cg.height
    let bytesPerRow = w * 4
    var data = [UInt8](repeating: 0, count: h * bytesPerRow)
    let cs = CGColorSpaceCreateDeviceRGB()
    guard let ctx = CGContext(data: &data, width: w, height: h, bitsPerComponent: 8,
                              bytesPerRow: bytesPerRow, space: cs,
                              bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else {
        return CGRect(x: 0, y: 0, width: w, height: h)
    }
    ctx.draw(cg, in: CGRect(x: 0, y: 0, width: w, height: h))
    var minX = w, minY = h, maxX = -1, maxY = -1
    for y in 0..<h {
        for x in 0..<w where data[y * bytesPerRow + x * 4 + 3] > 12 {
            if x < minX { minX = x }; if x > maxX { maxX = x }
            if y < minY { minY = y }; if y > maxY { maxY = y }
        }
    }
    guard maxX >= minX, maxY >= minY else { return CGRect(x: 0, y: 0, width: w, height: h) }
    return CGRect(x: minX, y: minY, width: maxX - minX + 1, height: maxY - minY + 1)
}

/// A white silhouette of `cg` (keeps its alpha shape, recolors opaque areas white).
func whiteSilhouette(_ cg: CGImage) -> CGImage {
    let w = cg.width, h = cg.height
    let cs = CGColorSpaceCreateDeviceRGB()
    guard let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8,
                              bytesPerRow: 0, space: cs,
                              bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return cg }
    ctx.setFillColor(CGColor(red: 1, green: 1, blue: 1, alpha: 1))
    ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
    ctx.setBlendMode(.destinationIn) // keep white only where the mark is opaque
    ctx.draw(cg, in: CGRect(x: 0, y: 0, width: w, height: h))
    return ctx.makeImage() ?? cg
}

guard let markImage = NSImage(contentsOfFile: markPath),
      let markCG = markImage.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
    fatalError("Cannot load logo mark at \(markPath)")
}
let trimmedMark = markCG.cropping(to: alphaBounds(markCG)) ?? markCG
let whiteMark = whiteSilhouette(trimmedMark)
let markAspect = CGFloat(whiteMark.width) / CGFloat(whiteMark.height)

// MARK: - Render one icon size

func render(_ size: Int) -> Data? {
    let s = CGFloat(size)
    let image = NSImage(size: NSSize(width: s, height: s))
    image.lockFocus()
    guard let ctx = NSGraphicsContext.current?.cgContext else { image.unlockFocus(); return nil }

    // Rounded macOS tile.
    let inset = s * 0.07
    let rect = CGRect(x: inset, y: inset, width: s - 2 * inset, height: s - 2 * inset)
    let radius = s * 0.225
    NSBezierPath(roundedRect: rect, xRadius: radius, yRadius: radius).addClip()

    // Brand purple gradient.
    let colors = [
        NSColor(srgbRed: 0.36, green: 0.30, blue: 0.92, alpha: 1).cgColor,
        NSColor(srgbRed: 0.58, green: 0.24, blue: 0.86, alpha: 1).cgColor,
    ] as CFArray
    if let grad = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(), colors: colors, locations: [0, 1]) {
        ctx.drawLinearGradient(grad, start: CGPoint(x: 0, y: s), end: CGPoint(x: s, y: 0), options: [])
    }

    // White logo mark, centered, fitted to ~62% of the tile (preserving aspect).
    let target = s * 0.62
    var mw = target, mh = target
    if markAspect >= 1 { mh = target / markAspect } else { mw = target * markAspect }
    let mrect = CGRect(x: (s - mw) / 2, y: (s - mh) / 2, width: mw, height: mh)
    ctx.interpolationQuality = .high
    ctx.draw(whiteMark, in: mrect)

    image.unlockFocus()
    guard let tiff = image.tiffRepresentation, let rep = NSBitmapImageRep(data: tiff) else { return nil }
    return rep.representation(using: .png, properties: [:])
}

// MARK: - Write the set + Contents.json

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
