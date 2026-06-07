import Foundation
import Observation

/// Shared, observable sync state for loading indicators across the UI.
@MainActor
@Observable
final class SyncCenter {
    var isSyncing = false
    var lastSyncedAt: Date?
}
