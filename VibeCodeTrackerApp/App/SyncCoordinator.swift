import Foundation
import SwiftData

/// Runs the full ingestion pipeline in order: detect projects → parse sessions →
/// read Git history. Used on launch and by the Refresh button.
@MainActor
enum SyncCoordinator {
    static func fullSync(context: ModelContext, center: SyncCenter? = nil) async {
        center?.isSyncing = true
        defer { center?.isSyncing = false }
        do {
            try await ClaudeProjectsScanner().scan(into: context)
            try await SessionSyncService().sync(into: context)
            try await GitSyncService().sync(into: context)
            try await BacklogSyncService().sync(into: context)
            try await StackSyncService().sync(into: context)
            try StatusSyncService().recomputeStatuses(in: context)
            try await TitleSyncService().syncTitles(in: context)
            center?.lastSyncedAt = .now
        } catch {
            Log.app.error("Full sync failed: \(error.localizedDescription, privacy: .public)")
        }
    }
}
