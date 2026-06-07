import Foundation

/// UserDefaults keys for user preferences (kept in one place).
enum PreferenceKey {
    static let syncViaICloud = "syncViaICloud"
    static let refreshFrequency = "refreshFrequency"
    static let showWorktrees = "showWorktrees"
    static let repositoryFilter = "repositoryFilter"
    // Project-detail section collapse state (global, not per-project).
    static let detailCommitsExpanded = "detailCommitsExpanded"
    static let detailSessionsExpanded = "detailSessionsExpanded"
    static let detailBacklogExpanded = "detailBacklogExpanded"
}

/// Sidebar repository filter.
enum RepositoryFilter: String, CaseIterable, Identifiable {
    case all
    case gitHub
    case localOnly

    var id: String { rawValue }

    var label: String {
        switch self {
        case .all: return "All repositories"
        case .gitHub: return "On GitHub"
        case .localOnly: return "Local only"
        }
    }
}

/// Pure filtering logic for the project sidebar (kept testable, no SwiftData).
enum ProjectFilter {
    static func matches(isWorktree: Bool, isOnGitHub: Bool, showWorktrees: Bool, repository: RepositoryFilter) -> Bool {
        if isWorktree && !showWorktrees { return false }
        switch repository {
        case .all: return true
        case .gitHub: return isOnGitHub
        case .localOnly: return !isOnGitHub
        }
    }
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
