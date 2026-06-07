import Foundation
import SwiftData

/// Recomputes `Session.status` for every session. Runs last in the pipeline so
/// that commits (needed for the blocked heuristic) are already loaded.
@MainActor
final class StatusSyncService {
    @discardableResult
    func recomputeStatuses(in context: ModelContext, now: Date = .now) throws -> Int {
        let sessions = try context.fetch(FetchDescriptor<Session>())
        var changed = 0
        for session in sessions {
            let commitDates = (session.project?.commits ?? []).map(\.authoredAt)
            let hasCommit = SessionStatusEvaluator.hasCommit(
                commitDates: commitDates,
                startedAt: session.startedAt,
                endedAt: session.endedAt
            )
            let newStatus = SessionStatusEvaluator.status(
                errorMessageCount: session.errorMessageCount,
                fileModifiedAt: session.fileModifiedAt,
                hasAssociatedCommit: hasCommit,
                now: now
            ).rawValue
            if session.status != newStatus {
                session.status = newStatus
                changed += 1
            }
        }
        if context.hasChanges { try context.save() }
        Log.parser.info("Status recompute updated \(changed) sessions.")
        return changed
    }
}
