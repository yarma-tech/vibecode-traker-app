import Foundation
import SwiftData

/// A project discovered on disk, before it touches the database.
/// Plain value type so discovery can run off the main thread.
struct DiscoveredProject: Equatable, Sendable {
    let claudeProjectHash: String
    let provisionalPath: String
    let provisionalName: String
    let lastActivityAt: Date
}

/// Summary of a scan, useful for logging and tests.
struct ScanSummary: Equatable, Sendable {
    var created: Int = 0
    var updated: Int = 0
    var total: Int = 0
}

/// Discovers Claude Code projects by listing the sub-folders of
/// `~/.claude/projects/` and upserts them into SwiftData.
///
/// Filesystem reads (`discoverProjects`) are pure and main-thread-safe to call
/// from a background task; the database upsert (`apply`) runs on the main actor.
final class ClaudeProjectsScanner {
    let projectsDirectory: URL
    private let fileManager: FileManager

    init(projectsDirectory: URL = ClaudeProjectsScanner.defaultProjectsDirectory,
         fileManager: FileManager = .default) {
        self.projectsDirectory = projectsDirectory
        self.fileManager = fileManager
    }

    static var defaultProjectsDirectory: URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appending(path: ".claude/projects", directoryHint: .isDirectory)
    }

    // MARK: - Pure decoding helpers

    /// Naive reconstruction of a filesystem path from a `~/.claude/projects/`
    /// folder name, which encodes the cwd by replacing "/" with "-".
    ///
    /// This is **lossy**: real dashes in path components are indistinguishable
    /// from separators (e.g. `/Users/x/agent-karata` and `/Users/x/agent/karata`
    /// both encode to the same name). The precise path is recovered from session
    /// `cwd` values in Slice 3; until then this is used for display only.
    static func decodePath(fromHash hash: String) -> String {
        guard !hash.isEmpty else { return "" }
        return hash.replacingOccurrences(of: "-", with: "/")
    }

    /// Best-effort display name from a (possibly provisional) path.
    static func displayName(forPath path: String) -> String {
        let trimmed = path.hasSuffix("/") ? String(path.dropLast()) : path
        let last = trimmed.split(separator: "/").last.map(String.init)
        guard let last, !last.isEmpty else { return "(unknown)" }
        return last
    }

    // MARK: - Discovery (filesystem, background-safe)

    /// Lists project folders and returns provisional metadata. Does not touch the DB.
    func discoverProjects() throws -> [DiscoveredProject] {
        guard fileManager.fileExists(atPath: projectsDirectory.path) else {
            Log.scanner.notice("Claude projects directory not found at \(self.projectsDirectory.path, privacy: .public)")
            return []
        }

        let keys: [URLResourceKey] = [.isDirectoryKey, .contentModificationDateKey]
        let entries = try fileManager.contentsOfDirectory(
            at: projectsDirectory,
            includingPropertiesForKeys: keys,
            options: [.skipsHiddenFiles]
        )

        var discovered: [DiscoveredProject] = []
        for entry in entries {
            let values = try? entry.resourceValues(forKeys: Set(keys))
            guard values?.isDirectory == true else { continue }

            let hash = entry.lastPathComponent
            let path = Self.decodePath(fromHash: hash)
            let modified = values?.contentModificationDate ?? Date(timeIntervalSince1970: 0)
            discovered.append(
                DiscoveredProject(
                    claudeProjectHash: hash,
                    provisionalPath: path,
                    provisionalName: Self.displayName(forPath: path),
                    lastActivityAt: modified
                )
            )
        }
        return discovered
    }

    // MARK: - Upsert (database, main actor)

    /// Inserts new projects and refreshes activity dates for known ones.
    /// Idempotent: re-running with the same input creates no duplicates.
    @MainActor
    @discardableResult
    func apply(_ discovered: [DiscoveredProject], into context: ModelContext) throws -> ScanSummary {
        let existing = try context.fetch(FetchDescriptor<Project>())
        var byHash: [String: Project] = [:]
        for project in existing { byHash[project.claudeProjectHash] = project }

        var summary = ScanSummary(total: discovered.count)
        for item in discovered {
            if let project = byHash[item.claudeProjectHash] {
                if item.lastActivityAt > project.lastActivityAt {
                    project.lastActivityAt = item.lastActivityAt
                }
                summary.updated += 1
            } else {
                let project = Project(
                    name: item.provisionalName,
                    path: item.provisionalPath,
                    claudeProjectHash: item.claudeProjectHash,
                    firstSeenAt: item.lastActivityAt,
                    lastActivityAt: item.lastActivityAt,
                    pathIsProvisional: true,
                    isWorktree: Project.detectIsWorktree(path: item.provisionalPath, claudeProjectHash: item.claudeProjectHash)
                )
                context.insert(project)
                byHash[item.claudeProjectHash] = project
                summary.created += 1
            }
        }
        if context.hasChanges { try context.save() }
        return summary
    }

    /// Convenience: discover off-main, then upsert on the main actor.
    @MainActor
    @discardableResult
    func scan(into context: ModelContext) async throws -> ScanSummary {
        let discovered = try await Task.detached(priority: .utility) { [self] in
            try discoverProjects()
        }.value
        let summary = try apply(discovered, into: context)
        Log.scanner.info("Scan complete: \(summary.created) new, \(summary.updated) existing, \(summary.total) total.")
        return summary
    }
}
