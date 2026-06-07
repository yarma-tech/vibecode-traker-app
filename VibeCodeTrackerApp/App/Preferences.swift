import Foundation

/// UserDefaults keys for user preferences (kept in one place).
enum PreferenceKey {
    static let syncViaICloud = "syncViaICloud"
    static let refreshFrequency = "refreshFrequency"
}

/// How often the app refreshes data in the background.
enum RefreshFrequency: String, CaseIterable, Identifiable {
    case manual
    case fifteenMinutes
    case hourly
    case daily

    var id: String { rawValue }

    var label: String {
        switch self {
        case .manual: return "Manual only"
        case .fifteenMinutes: return "Every 15 minutes"
        case .hourly: return "Every hour"
        case .daily: return "Once a day"
        }
    }

    /// Background refresh interval, or nil for manual-only.
    var interval: TimeInterval? {
        switch self {
        case .manual: return nil
        case .fifteenMinutes: return 15 * 60
        case .hourly: return 60 * 60
        case .daily: return 24 * 60 * 60
        }
    }
}
