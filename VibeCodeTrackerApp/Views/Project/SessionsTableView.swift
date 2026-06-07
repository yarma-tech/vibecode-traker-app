import SwiftUI

/// Compact, column-aligned table of a project's sessions.
/// Columns: status · prompt (≤40 chars) · model · tokens · duration · date.
struct SessionsTableView: View {
    let sessions: [Session]

    private let promptLimit = 40

    var body: some View {
        if sessions.isEmpty {
            Text("No sessions yet")
                .font(.caption)
                .foregroundStyle(.secondary)
        } else {
            Grid(alignment: .leading, horizontalSpacing: Spacing.md, verticalSpacing: Spacing.sm) {
                GridRow {
                    Text("").gridColumnAlignment(.center)
                    header("Prompt")
                    header("Model")
                    header("Tokens").gridColumnAlignment(.trailing)
                    header("Duration").gridColumnAlignment(.trailing)
                    header("When").gridColumnAlignment(.trailing)
                }
                Divider().gridCellColumns(6)
                ForEach(sessions) { session in
                    GridRow {
                        StatusBadge(status: session.status, showsLabel: false)
                        Text(preview(session.firstUserPrompt))
                            .lineLimit(1)
                        Text(session.modelFamily)
                            .foregroundStyle(.secondary)
                        Text(Format.tokens(session.totalTokens))
                            .foregroundStyle(.secondary)
                            .monospacedDigit()
                        Text(session.durationSeconds > 0 ? Format.duration(session.durationSeconds) : "—")
                            .foregroundStyle(.secondary)
                            .monospacedDigit()
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

    private func preview(_ prompt: String?) -> String {
        let text = (prompt ?? "(no prompt)").trimmingCharacters(in: .whitespacesAndNewlines)
        if text.count <= promptLimit { return text }
        return String(text.prefix(promptLimit - 1)) + "…"
    }
}
