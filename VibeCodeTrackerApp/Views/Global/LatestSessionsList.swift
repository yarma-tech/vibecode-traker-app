import SwiftUI

/// Cross-project list of the most recent sessions, with the project name shown.
struct LatestSessionsList: View {
    let sessions: [Session]

    var body: some View {
        Card(padding: 0) {
            VStack(alignment: .leading, spacing: 0) {
                ForEach(sessions) { session in
                    LatestSessionRow(session: session)
                    if session.id != sessions.last?.id {
                        Divider()
                    }
                }
            }
        }
    }
}

private struct LatestSessionRow: View {
    let session: Session

    var body: some View {
        HStack(spacing: 10) {
            StatusBadge(status: session.status, showsLabel: false)
            VStack(alignment: .leading, spacing: 2) {
                Text(session.firstUserPrompt ?? "(no prompt)")
                    .lineLimit(1)
                HStack(spacing: 8) {
                    Text(session.project?.name ?? "unknown")
                        .foregroundStyle(.primary)
                    Text("·")
                    Text(session.modelFamily)
                    Text("·")
                    Text("\(Format.tokens(session.totalTokens)) tok")
                }
                .font(.caption)
                .foregroundStyle(.secondary)
            }
            Spacer()
            Text(Format.relative(session.startedAt))
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }
}
