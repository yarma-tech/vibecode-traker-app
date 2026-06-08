import Foundation

/// Shared display formatting helpers (pure functions, unit-tested).
enum Format {
    /// Compact token counts: 950 -> "950", 1_234 -> "1.2k", 2_400_000 -> "2.4M".
    static func tokens(_ count: Int) -> String {
        let n = Double(count)
        switch abs(count) {
        case 1_000_000...:
            return trim(n / 1_000_000) + "M"
        case 1_000...:
            return trim(n / 1_000) + "k"
        default:
            return "\(count)"
        }
    }

    /// Human duration from seconds: 90 -> "2 min", 3_600 -> "1h 0m", 30 -> "30s".
    static func duration(_ seconds: Int) -> String {
        if seconds < 60 { return "\(seconds)s" }
        let minutes = seconds / 60
        if minutes < 60 { return "\(minutes) min" }
        let hours = minutes / 60
        let remMinutes = minutes % 60
        return "\(hours)h \(remMinutes)m"
    }

    private static let relativeFormatter: RelativeDateTimeFormatter = {
        let f = RelativeDateTimeFormatter()
        f.unitsStyle = .abbreviated
        return f
    }()

    /// Relative time, e.g. "2h ago". `now` is injectable for tests.
    static func relative(_ date: Date, now: Date = .now) -> String {
        relativeFormatter.localizedString(for: date, relativeTo: now)
    }

    /// Single-line preview of a user prompt, truncated to `limit` characters
    /// with a trailing ellipsis. `nil` renders as "(no prompt)".
    static func promptPreview(_ prompt: String?, limit: Int = 40) -> String {
        let text = (prompt ?? "(no prompt)").trimmingCharacters(in: .whitespacesAndNewlines)
        if text.count <= limit { return text }
        return String(text.prefix(limit - 1)) + "…"
    }

    private static func trim(_ value: Double) -> String {
        // One decimal, but drop a trailing ".0".
        let s = String(format: "%.1f", value)
        return s.hasSuffix(".0") ? String(s.dropLast(2)) : s
    }
}
