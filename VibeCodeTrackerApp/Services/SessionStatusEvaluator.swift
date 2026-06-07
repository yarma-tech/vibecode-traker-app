import Foundation

/// Pure session-status heuristic (PRD §7).
///
/// - `inProgress`: source file modified within the last 30 minutes.
/// - `blocked`: ≥ 3 assistant messages contain error keywords AND no commit is
///   associated with the session's time window.
/// - `completed`: everything else.
enum SessionStatusEvaluator {
    static let inProgressWindow: TimeInterval = 30 * 60
    static let blockedErrorThreshold = 3

    static func status(
        errorMessageCount: Int,
        fileModifiedAt: Date,
        hasAssociatedCommit: Bool,
        now: Date
    ) -> SessionStatus {
        if now.timeIntervalSince(fileModifiedAt) < inProgressWindow {
            return .inProgress
        }
        if errorMessageCount >= blockedErrorThreshold && !hasAssociatedCommit {
            return .blocked
        }
        return .completed
    }

    /// Whether any commit falls within [startedAt, endedAt + 5 min].
    /// Used as a V1 proxy for "a commit is associated with the session" since
    /// explicit session↔commit matching is a V2 refinement.
    static func hasCommit(commitDates: [Date], startedAt: Date, endedAt: Date?) -> Bool {
        let upper = (endedAt ?? startedAt).addingTimeInterval(5 * 60)
        return commitDates.contains { $0 >= startedAt && $0 <= upper }
    }
}
