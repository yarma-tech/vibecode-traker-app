import SwiftUI

/// Plain, `Sendable` projection of a `BacklogItem` for display.
///
/// The view renders these value snapshots instead of live `@Model` objects. The
/// background sync deletes `BacklogItem`s on every pass (vanished TODO lines); a
/// `ForEach` holding the deleted models would trap on the next render pass
/// ("backing data could no longer be found"). Snapshotting severs that link.
struct BacklogRowData: Identifiable, Equatable, Sendable {
    let id: UUID
    let title: String
    let isDone: Bool
    let sourceFile: String
    let lineNumber: Int
    let priority: String?

    /// Sort rank: P0=0, P1=1, P2=2, none=3.
    var priorityRank: Int {
        switch priority {
        case "P0": return 0
        case "P1": return 1
        case "P2": return 2
        default: return 3
        }
    }

    init(id: UUID, title: String, isDone: Bool, sourceFile: String, lineNumber: Int, priority: String?) {
        self.id = id
        self.title = title
        self.isDone = isDone
        self.sourceFile = sourceFile
        self.lineNumber = lineNumber
        self.priority = priority
    }

    init(_ item: BacklogItem) {
        self.init(id: item.id, title: item.title, isDone: item.isDone,
                  sourceFile: item.sourceFile, lineNumber: item.lineNumber, priority: item.priority)
    }

    /// Open items first, then by priority, then by source line.
    static func sorted(_ items: [BacklogRowData]) -> [BacklogRowData] {
        items.sorted { lhs, rhs in
            if lhs.isDone != rhs.isDone { return !lhs.isDone }      // open first
            if lhs.priorityRank != rhs.priorityRank { return lhs.priorityRank < rhs.priorityRank }
            return lhs.lineNumber < rhs.lineNumber
        }
    }
}

/// Read-only backlog list parsed from TODO.md / BACKLOG.md.
struct BacklogView: View {
    let items: [BacklogRowData]

    private var sorted: [BacklogRowData] { BacklogRowData.sorted(items) }

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
    let item: BacklogRowData

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
