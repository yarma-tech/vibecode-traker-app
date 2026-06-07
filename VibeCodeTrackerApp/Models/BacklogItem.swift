import Foundation
import SwiftData

/// A checkbox item parsed from a project's TODO.md / BACKLOG.md.
@Model
final class BacklogItem {
    @Attribute(.unique) var id: UUID
    var title: String
    var isDone: Bool
    var sourceFile: String       // e.g. "TODO.md"
    var lineNumber: Int
    var lastSeenAt: Date
    var priority: String?        // "P0" | "P1" | "P2"

    var project: Project?

    init(
        id: UUID = UUID(),
        title: String,
        isDone: Bool,
        sourceFile: String,
        lineNumber: Int,
        lastSeenAt: Date = .now,
        priority: String? = nil,
        project: Project? = nil
    ) {
        self.id = id
        self.title = title
        self.isDone = isDone
        self.sourceFile = sourceFile
        self.lineNumber = lineNumber
        self.lastSeenAt = lastSeenAt
        self.priority = priority
        self.project = project
    }
}
