import SwiftUI

/// Read-only backlog list parsed from TODO.md / BACKLOG.md.
struct BacklogView: View {
    let items: [BacklogItem]

    private var sorted: [BacklogItem] {
        items.sorted { lhs, rhs in
            if lhs.isDone != rhs.isDone { return !lhs.isDone }      // open first
            if lhs.priorityRank != rhs.priorityRank { return lhs.priorityRank < rhs.priorityRank }
            return lhs.lineNumber < rhs.lineNumber
        }
    }

    var body: some View {
        if items.isEmpty {
            Text("No TODO.md or BACKLOG.md found")
                .font(.caption)
                .foregroundStyle(.secondary)
        } else {
            VStack(alignment: .leading, spacing: 0) {
                ForEach(sorted) { item in
                    BacklogRow(item: item)
                    if item.id != sorted.last?.id { Divider() }
                }
            }
            .background(.background.secondary, in: RoundedRectangle(cornerRadius: 12))
            .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(.separator, lineWidth: 0.5))
        }
    }
}

private struct BacklogRow: View {
    let item: BacklogItem

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: item.isDone ? "checkmark.square.fill" : "square")
                .foregroundStyle(item.isDone ? .green : .secondary)
            Text(item.title)
                .strikethrough(item.isDone, color: .secondary)
                .foregroundStyle(item.isDone ? .secondary : .primary)
                .lineLimit(1)
            Spacer()
            if let priority = item.priority {
                Text(priority)
                    .font(.caption2.weight(.bold))
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(priorityColor(priority).opacity(0.18), in: Capsule())
                    .foregroundStyle(priorityColor(priority))
            }
            Text(item.sourceFile)
                .font(.caption2)
                .foregroundStyle(.tertiary)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 7)
    }

    private func priorityColor(_ priority: String) -> Color {
        switch priority {
        case "P0": return .red
        case "P1": return .orange
        default: return .secondary
        }
    }
}

extension BacklogItem {
    /// Sort rank: P0=0, P1=1, P2=2, none=3.
    var priorityRank: Int {
        switch priority {
        case "P0": return 0
        case "P1": return 1
        case "P2": return 2
        default: return 3
        }
    }
}
