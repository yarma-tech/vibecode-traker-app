import Foundation

/// CloudKit configuration constants and sync-status presentation.
///
/// CloudKit is **not active** in V1. Enabling it requires (see BLOCKERS.md):
///  1. An Apple Developer account and the `iCloud.tech.yannick.vibecodetracker`
///     container configured in the app's iCloud entitlement.
///  2. Removing every `@Attribute(.unique)` from the models — CloudKit-backed
///     SwiftData does not support unique constraints — and replacing the upsert
///     logic's reliance on them with manual de-duplication.
/// Until then the app runs purely locally and degrades gracefully.
enum CloudKitSupport {
    static let containerID = "iCloud.tech.yannick.vibecodetracker"

    /// Whether a CloudKit-backed store can actually be created in this build.
    /// Always false in V1 (no entitlement / schema not CloudKit-compatible).
    static let isAvailable = false
}

/// High-level sync status shown in the sidebar.
enum SyncStatus: Equatable {
    case localOnly
    case cloudSetupRequired
    case synced(Date)

    var label: String {
        switch self {
        case .localOnly: return "Local only"
        case .cloudSetupRequired: return "iCloud (setup required)"
        case .synced(let date): return "Synced \(Format.relative(date))"
        }
    }

    var systemImage: String {
        switch self {
        case .localOnly: return "internaldrive"
        case .cloudSetupRequired: return "exclamationmark.icloud"
        case .synced: return "checkmark.icloud"
        }
    }

    /// Resolves the status from the user's intent and actual availability.
    static func resolve(syncEnabled: Bool, cloudKitAvailable: Bool = CloudKitSupport.isAvailable) -> SyncStatus {
        guard syncEnabled else { return .localOnly }
        return cloudKitAvailable ? .synced(.now) : .cloudSetupRequired
    }
}
