import SwiftUI

/// Recent commits list with an inferred-type badge.
struct CommitsListView: View {
    let commits: [Commit]

    var body: some View {
        if commits.isEmpty {
            Text("No commits in the last 90 days")
                .font(.caption)
                .foregroundStyle(.secondary)
        } else {
            VStack(alignment: .leading, spacing: 0) {
                ForEach(commits) { commit in
                    CommitRow(commit: commit)
                    if commit.id != commits.last?.id { Divider() }
                }
            }
            .background(.background.secondary, in: RoundedRectangle(cornerRadius: 12))
            .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(.separator, lineWidth: 0.5))
        }
    }
}

private struct CommitRow: View {
    let commit: Commit

    var body: some View {
        HStack(spacing: 10) {
            CommitTypeBadge(type: commit.inferredType ?? "chore")
            Text(commit.message)
                .lineLimit(1)
            Spacer()
            Text(Format.relative(commit.authoredAt))
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 7)
    }
}

/// Small colored badge for a commit type (feat/fix/etc.).
struct CommitTypeBadge: View {
    let type: String

    private var color: Color {
        switch type {
        case "feat": return .green
        case "fix": return .red
        case "refactor": return .purple
        case "docs": return .blue
        case "test": return .orange
        case "perf": return .pink
        default: return .gray
        }
    }

    var body: some View {
        Text(type)
            .font(.caption2.weight(.semibold).monospaced())
            .frame(width: 58)
            .padding(.vertical, 2)
            .background(color.opacity(0.18), in: RoundedRectangle(cornerRadius: 4))
            .foregroundStyle(color)
    }
}
