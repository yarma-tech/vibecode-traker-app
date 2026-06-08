import SwiftUI

/// Compact, column-aligned table of a project's sessions.
/// Columns: status · title + prompt (≤40 chars) · model · tokens · duration · commit · date.
struct SessionsTableView: View {
    let sessions: [Session]
    /// Authored dates of the project's commits, used to mark each session
    /// "Commit" / "No commit" via `SessionStatusEvaluator.hasCommit`.
    var commitDates: [Date] = []

    var body: some View {
        if sessions.isEmpty {
            Text("No sessions yet")
                .font(.caption)
                .foregroundStyle(.secondary)
        } else {
            Grid(alignment: .leading, horizontalSpacing: Spacing.md, verticalSpacing: Spacing.sm) {
                GridRow {
                    Text("").gridColumnAlignment(.center)
                    header("Session")
                    header("Model")
                    header("Tokens").gridColumnAlignment(.trailing)
                    header("Duration").gridColumnAlignment(.trailing)
                    header("Commit")
                    header("When").gridColumnAlignment(.trailing)
                }
                Divider().gridCellColumns(7)
                ForEach(sessions) { session in
                    GridRow {
                        StatusBadge(status: session.status, showsLabel: false)
                        titleCell(session)
                        Text(session.modelFamily)
                            .foregroundStyle(.secondary)
                        Text(Format.tokens(session.totalTokens))
                            .foregroundStyle(.secondary)
                            .monospacedDigit()
                        Text(session.durationSeconds > 0 ? Format.duration(session.durationSeconds) : "—")
                            .foregroundStyle(.secondary)
                            .monospacedDigit()
                        SessionCommitBadge(committed: committed(session))
                        Text(Format.relative(session.startedAt))
                            .foregroundStyle(.secondary)
                    }
                    .font(.callout)
                }
            }
        }
    }

    private func header(_ text: String) -> some View {
        Text(text.uppercased())
            .font(.caption2.weight(.semibold))
            .foregroundStyle(.secondary)
    }

    /// Synthesized title on top, the 40-char prompt preview beneath it.
    /// Until a title is generated, the prompt is shown as the primary line.
    @ViewBuilder
    private func titleCell(_ session: Session) -> some View {
        VStack(alignment: .leading, spacing: 1) {
            Text(session.title ?? Format.promptPreview(session.firstUserPrompt))
                .fontWeight(.semibold)
                .lineLimit(1)
            if session.title != nil {
                Text(Format.promptPreview(session.firstUserPrompt))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
        }
    }

    /// A session is "committed" if a project commit falls within its time window
    /// (the same proxy used to evaluate session status).
    private func committed(_ session: Session) -> Bool {
        SessionStatusEvaluator.hasCommit(commitDates: commitDates,
                                         startedAt: session.startedAt,
                                         endedAt: session.endedAt)
    }
}
