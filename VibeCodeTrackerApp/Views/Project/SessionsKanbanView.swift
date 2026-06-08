import SwiftUI

/// Pure grouping of sessions into the fixed kanban column order.
/// Extracted from the view so the logic is unit-testable without SwiftData.
enum SessionKanbanGrouping {
    /// Fixed column order; every column is always present (even if empty).
    static let order: [SessionStatus] = [.inProgress, .blocked, .completed]

    /// Generic core — group any items by status, newest-first within a column.
    /// Testable with plain value types (no `@Model` needed).
    static func group<T>(
        _ items: [T],
        status: (T) -> SessionStatus,
        startedAt: (T) -> Date
    ) -> [(status: SessionStatus, items: [T])] {
        let byStatus = Dictionary(grouping: items, by: status)
        return order.map { status in
            let group = (byStatus[status] ?? []).sorted { startedAt($0) > startedAt($1) }
            return (status, group)
        }
    }

    /// Convenience for `Session`: maps the stored status string, with unknown
    /// values falling back to `.completed`.
    static func group(_ sessions: [Session]) -> [(status: SessionStatus, items: [Session])] {
        group(sessions,
              status: { SessionStatus(rawValue: $0.status) ?? .completed },
              startedAt: { $0.startedAt })
    }
}

/// Read-only kanban board of a project's sessions, grouped by lifecycle status.
/// Status is derived (not user-set), so there is no drag-to-change.
struct SessionsKanbanView: View {
    let sessions: [Session]
    var commitDates: [Date] = []

    var body: some View {
        HStack(alignment: .top, spacing: Spacing.md) {
            ForEach(SessionKanbanGrouping.group(sessions), id: \.status) { column in
                KanbanColumn(status: column.status,
                             sessions: column.items,
                             commitDates: commitDates)
                    .frame(maxWidth: .infinity, alignment: .top)
            }
        }
    }
}

private struct KanbanColumn: View {
    let status: SessionStatus
    let sessions: [Session]
    let commitDates: [Date]

    var body: some View {
        VStack(alignment: .leading, spacing: Spacing.sm) {
            HStack(spacing: Spacing.sm) {
                StatusBadge(status: status.rawValue)
                Text("\(sessions.count)")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 1)
                    .background(.background.secondary, in: Capsule())
                Spacer(minLength: 0)
            }

            if sessions.isEmpty {
                Text("No sessions")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.vertical, Spacing.sm)
            } else {
                LazyVStack(alignment: .leading, spacing: Spacing.sm) {
                    ForEach(sessions) { session in
                        SessionCard(session: session, commitDates: commitDates)
                    }
                }
            }
        }
    }
}

private struct SessionCard: View {
    let session: Session
    let commitDates: [Date]

    private var committed: Bool {
        SessionStatusEvaluator.hasCommit(commitDates: commitDates,
                                         startedAt: session.startedAt,
                                         endedAt: session.endedAt)
    }

    var body: some View {
        Card {
            VStack(alignment: .leading, spacing: Spacing.xs) {
                HStack {
                    StatusBadge(status: session.status, showsLabel: false)
                    Spacer()
                    Text(Format.relative(session.startedAt))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }

                Text(session.title ?? Format.promptPreview(session.firstUserPrompt))
                    .font(.headline)
                    .lineLimit(2)

                if session.title != nil {
                    Text(Format.promptPreview(session.firstUserPrompt))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }

                HStack(spacing: Spacing.sm) {
                    Text(session.modelFamily)
                    Text("·")
                    Text("\(Format.tokens(session.totalTokens)) tok")
                    Spacer()
                    SessionCommitBadge(committed: committed)
                }
                .font(.caption)
                .foregroundStyle(.secondary)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}
