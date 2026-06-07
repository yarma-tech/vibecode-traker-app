import Foundation
import SwiftData

/// A Git commit read from a project's repository.
///
/// V1 omits the `Commit ↔ Session` link from the PRD data model — session/commit
/// matching is a documented V2 refinement. The reverse hint already exists via
/// `Session.commitHashes`.
@Model
final class Commit {
    @Attribute(.unique) var sha: String
    var message: String
    var authorName: String
    var authoredAt: Date
    var filesChanged: Int
    var insertions: Int
    var deletions: Int
    var branch: String?
    /// "feat" | "fix" | "refactor" | "docs" | "chore" | "test" | ...
    var inferredType: String?

    var project: Project?

    init(
        sha: String,
        message: String,
        authorName: String,
        authoredAt: Date,
        filesChanged: Int = 0,
        insertions: Int = 0,
        deletions: Int = 0,
        branch: String? = nil,
        inferredType: String? = nil,
        project: Project? = nil
    ) {
        self.sha = sha
        self.message = message
        self.authorName = authorName
        self.authoredAt = authoredAt
        self.filesChanged = filesChanged
        self.insertions = insertions
        self.deletions = deletions
        self.branch = branch
        self.inferredType = inferredType
        self.project = project
    }
}
