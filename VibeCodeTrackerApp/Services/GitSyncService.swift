import Foundation
import SwiftData

/// Commits read for one project repository.
struct RepoCommits: Sendable {
    let claudeProjectHash: String
    let branch: String?
    let commits: [GitCommit]
}

struct GitSyncSummary: Equatable, Sendable {
    var commitsCreated = 0
    var commitsUpdated = 0
    var reposScanned = 0
}

/// Reads Git history for every project with a known on-disk path and upserts
/// `Commit` rows. Git execution happens off the main actor; the upsert is on it.
final class GitSyncService {

    // MARK: - Reading (background-safe)

    func readAll(candidates: [(hash: String, path: String)]) -> [RepoCommits] {
        let inspector = GitInspector()
        var result: [RepoCommits] = []
        for candidate in candidates {
            guard GitInspector.isGitRepository(candidate.path) else { continue }
            let commits = (try? inspector.commits(inRepository: candidate.path)) ?? []
            guard !commits.isEmpty else { continue }
            let branch = inspector.currentBranch(inRepository: candidate.path)
            result.append(RepoCommits(claudeProjectHash: candidate.hash, branch: branch, commits: commits))
        }
        return result
    }

    // MARK: - Upsert (main actor)

    @MainActor
    @discardableResult
    func apply(_ repos: [RepoCommits], into context: ModelContext) throws -> GitSyncSummary {
        var projectsByHash: [String: Project] = [:]
        for project in try context.fetch(FetchDescriptor<Project>()) {
            projectsByHash[project.claudeProjectHash] = project
        }
        var commitsBySha: [String: Commit] = [:]
        for commit in try context.fetch(FetchDescriptor<Commit>()) {
            commitsBySha[commit.sha] = commit
        }

        var summary = GitSyncSummary()
        for repo in repos {
            guard let project = projectsByHash[repo.claudeProjectHash] else { continue }
            summary.reposScanned += 1
            for gitCommit in repo.commits {
                if let existing = commitsBySha[gitCommit.sha] {
                    existing.branch = repo.branch
                    summary.commitsUpdated += 1
                } else {
                    let commit = Commit(
                        sha: gitCommit.sha,
                        message: gitCommit.message,
                        authorName: gitCommit.authorName,
                        authoredAt: gitCommit.authoredAt,
                        filesChanged: gitCommit.filesChanged,
                        insertions: gitCommit.insertions,
                        deletions: gitCommit.deletions,
                        branch: repo.branch,
                        inferredType: gitCommit.inferredType,
                        project: project
                    )
                    context.insert(commit)
                    commitsBySha[gitCommit.sha] = commit
                    summary.commitsCreated += 1
                }
            }
        }
        if context.hasChanges { try context.save() }
        return summary
    }

    /// Convenience: gather candidate repos (main), read Git (background), upsert (main).
    @MainActor
    @discardableResult
    func sync(into context: ModelContext) async throws -> GitSyncSummary {
        let candidates: [(hash: String, path: String)] = try context.fetch(FetchDescriptor<Project>())
            .filter { !$0.pathIsProvisional }
            .map { (hash: $0.claudeProjectHash, path: $0.path) }

        let repos = await Task.detached(priority: .utility) { [self] in
            readAll(candidates: candidates)
        }.value

        let summary = try apply(repos, into: context)
        Log.git.info("Git sync: \(summary.commitsCreated) new commits across \(summary.reposScanned) repos.")
        return summary
    }
}
