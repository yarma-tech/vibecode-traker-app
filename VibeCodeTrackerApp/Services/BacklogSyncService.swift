import Foundation
import SwiftData

/// Parsed backlog for one project.
struct ProjectBacklog: Sendable {
    let claudeProjectHash: String
    let items: [ParsedBacklogItem]
}

struct BacklogSyncSummary: Equatable, Sendable {
    var created = 0
    var updated = 0
    var removed = 0
}

/// Parses TODO.md / BACKLOG.md for each project and reconciles the database
/// (add new items, update changed ones, remove items no longer present).
final class BacklogSyncService {
    private let parser = BacklogParser()

    static func naturalKey(sourceFile: String, title: String) -> String {
        "\(sourceFile)|\(title)"
    }

    // MARK: - Parsing (background-safe)

    func parseAll(candidates: [(hash: String, path: String)]) -> [ProjectBacklog] {
        candidates.map { candidate in
            ProjectBacklog(
                claudeProjectHash: candidate.hash,
                items: parser.parseProject(at: candidate.path)
            )
        }
    }

    // MARK: - Reconcile (main actor)

    @MainActor
    @discardableResult
    func apply(_ backlogs: [ProjectBacklog], into context: ModelContext, now: Date = .now) throws -> BacklogSyncSummary {
        var projectsByHash: [String: Project] = [:]
        for project in try context.fetch(FetchDescriptor<Project>()) {
            projectsByHash[project.claudeProjectHash] = project
        }

        var summary = BacklogSyncSummary()
        for backlog in backlogs {
            guard let project = projectsByHash[backlog.claudeProjectHash] else { continue }

            var existing: [String: BacklogItem] = [:]
            for item in project.backlogItems {
                existing[Self.naturalKey(sourceFile: item.sourceFile, title: item.title)] = item
            }

            var seen = Set<String>()
            for parsed in backlog.items {
                let key = Self.naturalKey(sourceFile: parsed.sourceFile, title: parsed.title)
                seen.insert(key)
                if let item = existing[key] {
                    item.isDone = parsed.isDone
                    item.priority = parsed.priority
                    item.lineNumber = parsed.lineNumber
                    item.lastSeenAt = now
                    summary.updated += 1
                } else {
                    let item = BacklogItem(
                        title: parsed.title,
                        isDone: parsed.isDone,
                        sourceFile: parsed.sourceFile,
                        lineNumber: parsed.lineNumber,
                        lastSeenAt: now,
                        priority: parsed.priority,
                        project: project
                    )
                    context.insert(item)
                    summary.created += 1
                }
            }

            // Remove items that have disappeared from the files.
            for (key, item) in existing where !seen.contains(key) {
                context.delete(item)
                summary.removed += 1
            }
        }

        if context.hasChanges { try context.save() }
        return summary
    }

    @MainActor
    @discardableResult
    func sync(into context: ModelContext) async throws -> BacklogSyncSummary {
        let candidates: [(hash: String, path: String)] = try context.fetch(FetchDescriptor<Project>())
            .filter { !$0.pathIsProvisional }
            .map { (hash: $0.claudeProjectHash, path: $0.path) }

        let backlogs = await Task.detached(priority: .utility) { [self] in
            parseAll(candidates: candidates)
        }.value

        let summary = try apply(backlogs, into: context)
        Log.backlog.info("Backlog sync: +\(summary.created) ~\(summary.updated) -\(summary.removed).")
        return summary
    }
}
