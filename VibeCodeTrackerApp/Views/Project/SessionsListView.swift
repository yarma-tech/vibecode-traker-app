import SwiftUI

/// List of a project's sessions, most recent first.
struct SessionsListView: View {
    let sessions: [Session]

    var body: some View {
        if sessions.isEmpty {
            ContentUnavailableView(
                "No sessions yet",
                systemImage: "bubble.left.and.bubble.right",
                description: Text("Sessions appear here once Claude Code transcripts are parsed.")
            )
        } else {
            List(sessions) { session in
                SessionRow(session: session)
            }
        }
    }
}

/// Single session row: prompt preview + model / tokens / time metadata.
struct SessionRow: View {
    let session: Session

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(session.firstUserPrompt ?? "(no prompt)")
                .font(.body)
                .lineLimit(1)
            HStack(spacing: 10) {
                StatusBadge(status: session.status)
                Label(session.modelFamily, systemImage: "cpu")
                Label("\(Format.tokens(session.totalTokens)) tok", systemImage: "number")
                if session.durationSeconds > 0 {
                    Label(Format.duration(session.durationSeconds), systemImage: "clock")
                }
                Spacer()
                Text(Format.relative(session.startedAt))
            }
            .font(.caption)
            .foregroundStyle(.secondary)
            .labelStyle(.titleAndIcon)
        }
        .padding(.vertical, 2)
    }
}
