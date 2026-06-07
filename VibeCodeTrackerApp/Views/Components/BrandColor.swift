import SwiftUI

/// Brand palette derived from the Vibe Code Tracker logo.
///
/// `brand` is the core logo purple (#7030F8). It is kept in sync with the
/// `AccentColor` asset (same sRGB components) so the system tint and explicit
/// brand usages match. The light/dark variants are for tints and pressed states.
extension Color {
    /// Core logo purple — #7030F8 / rgb(112, 48, 248). Mirrors `AccentColor`.
    static let brand = Color(red: 112 / 255, green: 48 / 255, blue: 248 / 255)

    /// Deep variant — #5018F8 — for pressed/active states.
    static let brandDeep = Color(red: 80 / 255, green: 24 / 255, blue: 248 / 255)

    /// Light tint — #C8B0F8 — for subtle fills and highlights.
    static let brandTint = Color(red: 200 / 255, green: 176 / 255, blue: 248 / 255)
}
