import SwiftUI

/// Branch icon + label marking whether a session produced a commit
/// (within its time window — the same proxy used for session status).
/// Shared by the sessions table and the kanban card.
struct SessionCommitBadge: View {
    let committed: Bool

    var body: some View {
        HStack(spacing: Spacing.xs) {
            Image(systemName: committed ? "arrow.triangle.branch" : "minus")
                .foregroundStyle(committed ? Color.green : Color.secondary)
            Text(committed ? "Commit" : "No commit")
                .foregroundStyle(.secondary)
        }
    }
}
