import Foundation
import SwiftData

/// A development project that has had at least one Claude Code session.
///
/// Identified primarily by `claudeProjectHash` — the folder name under
/// `~/.claude/projects/` — which is a stable key even though it cannot be
/// losslessly decoded back into a filesystem path. The real `path` is recovered
/// from session `cwd` values (Slice 3).
///
/// Relationships to `Session`, `Commit`, and `BacklogItem` are added in the
/// slices that introduce those models.
@Model
final class Project {
    @Attribute(.unique) var id: UUID
    var name: String
    var path: String
    /// Folder name under `~/.claude/projects/` (the encoded cwd). Stable identity.
    @Attribute(.unique) var claudeProjectHash: String
    var stack: [String]
    var firstSeenAt: Date
    var lastActivityAt: Date
    var isArchived: Bool
    var isPinned: Bool
    /// Optional hex color (e.g. "#5A5AE5") for visual distinction in the UI.
    var customColor: String?
    /// True until the real path is recovered from a session `cwd` (Slice 3).
    /// While provisional, `path`/`name` come from the lossy folder-name decode.
    var pathIsProvisional: Bool
    /// True if this is a Claude Code git worktree (under `.claude/worktrees/`),
    /// not a real top-level project. Hidden from the list/KPIs by default.
    var isWorktree: Bool
    /// True if a Git remote points at github.com (public or private).
    var isOnGitHub: Bool
    /// First Git remote URL, if any (informational).
    var remoteURL: String?

    @Relationship(deleteRule: .cascade, inverse: \Session.project)
    var sessions: [Session] = []

    @Relationship(deleteRule: .cascade, inverse: \Commit.project)
    var commits: [Commit] = []

    @Relationship(deleteRule: .cascade, inverse: \BacklogItem.project)
    var backlogItems: [BacklogItem] = []

    init(
        id: UUID = UUID(),
        name: String,
        path: String,
        claudeProjectHash: String,
        stack: [String] = [],
        firstSeenAt: Date = .now,
        lastActivityAt: Date = .now,
        isArchived: Bool = false,
        isPinned: Bool = false,
        customColor: String? = nil,
        pathIsProvisional: Bool = true,
        isWorktree: Bool = false,
        isOnGitHub: Bool = false,
        remoteURL: String? = nil
    ) {
        self.id = id
        self.name = name
        self.path = path
        self.claudeProjectHash = claudeProjectHash
        self.stack = stack
        self.firstSeenAt = firstSeenAt
        self.lastActivityAt = lastActivityAt
        self.isArchived = isArchived
        self.isPinned = isPinned
        self.customColor = customColor
        self.pathIsProvisional = pathIsProvisional
        self.isWorktree = isWorktree
        self.isOnGitHub = isOnGitHub
        self.remoteURL = remoteURL
    }

    /// Detects a Claude Code worktree from its path or folder hash.
    static func detectIsWorktree(path: String, claudeProjectHash: String) -> Bool {
        path.contains("/.claude/worktrees/") || claudeProjectHash.contains("claude-worktrees")
    }
}
