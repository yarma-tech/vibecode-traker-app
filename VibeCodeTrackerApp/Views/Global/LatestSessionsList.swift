import SwiftUI

/// Cross-project list of the most recent sessions, with the project name shown.
struct LatestSessionsList: View {
    let sessions: [Session]

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            ForEach(sessions) { session in
                LatestSessionRow(session: session)
                if session.id != sessions.last?.id {
                    Divider()
                }
            }
        }
        .background(.background.secondary, in: RoundedRectangle(cornerRadius: 12))
        .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(.separator, lineWidth: 0.5))
    }
}

private struct LatestSessionRow: View {
    let session: Session

    var body: some View {
        HStack(spacing: 10) {
            StatusDot(status: session.status)
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

/// Small colored status indicator (a fuller badge arrives in Slice 9).
struct StatusDot: View {
    let status: String

    private var color: Color {
        switch status {
        case SessionStatus.inProgress.rawValue: return .yellow
        case SessionStatus.blocked.rawValue: return .red
        default: return .green
        }
    }

    var body: some View {
        Circle().fill(color).frame(width: 8, height: 8)
    }
}
