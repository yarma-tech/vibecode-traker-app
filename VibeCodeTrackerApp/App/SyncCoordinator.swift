import Foundation
import SwiftData

/// Runs the full ingestion pipeline in order: detect projects → parse sessions →
/// read Git history. Used on launch and by the Refresh button.
@MainActor
enum SyncCoordinator {
    static func fullSync(context: ModelContext) async {
        do {
            try await ClaudeProjectsScanner().scan(into: context)
            try await SessionSyncService().sync(into: context)
            try await GitSyncService().sync(into: context)
            try await BacklogSyncService().sync(into: context)
            try await StackSyncService().sync(into: context)
            try StatusSyncService().recomputeStatuses(in: context)
            // No-op unless an Anthropic Admin key is in the Keychain.
            try await UsageSyncService().sync(into: context)
        } catch {
            Log.app.error("Full sync failed: \(error.localizedDescription, privacy: .public)")
        }
    }
}
