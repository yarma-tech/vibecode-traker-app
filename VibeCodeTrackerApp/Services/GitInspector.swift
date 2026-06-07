import Foundation

/// A commit parsed from `git log` output.
struct GitCommit: Equatable, Sendable {
    var sha: String
    var authorName: String
    var authoredAt: Date
    var message: String
    var filesChanged: Int
    var insertions: Int
    var deletions: Int
    var inferredType: String
}

/// Reads commits from a Git repository by shelling out to `/usr/bin/git`.
///
/// All methods are synchronous and blocking; call them from a background task.
final class GitInspector {
    private let gitPath: String

    init(gitPath: String = "/usr/bin/git") {
        self.gitPath = gitPath
    }

    // MARK: - Repository queries

    static func isGitRepository(_ path: String) -> Bool {
        // `.git` is a directory in a normal clone, or a file in a worktree.
        FileManager.default.fileExists(atPath: (path as NSString).appendingPathComponent(".git"))
    }

    func commits(inRepository path: String, since: String = "90 days ago", limit: Int = 500) throws -> [GitCommit] {
        let output = try run([
            "-C", path,
            "log",
            "--since=\(since)",
            "--max-count=\(limit)",
            "--pretty=format:%H|%an|%ai|%s",
            "--shortstat"
        ])
        return Self.parseLog(output)
    }

    func currentBranch(inRepository path: String) -> String? {
        let output = (try? run(["-C", path, "branch", "--show-current"])) ?? ""
        let trimmed = output.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }

    // MARK: - Pure parsing (unit-tested)

    static func parseLog(_ output: String) -> [GitCommit] {
        var commits: [GitCommit] = []
        output.enumerateLines { line, _ in
            if let header = parseHeader(line) {
                commits.append(header)
            } else if line.contains(" changed"), !commits.isEmpty {
                let stats = parseShortstat(line)
                commits[commits.count - 1].filesChanged = stats.files
                commits[commits.count - 1].insertions = stats.insertions
                commits[commits.count - 1].deletions = stats.deletions
            }
        }
        return commits
    }

    static func parseHeader(_ line: String) -> GitCommit? {
        let parts = line.split(separator: "|", maxSplits: 3, omittingEmptySubsequences: false).map(String.init)
        guard parts.count == 4 else { return nil }
        let sha = parts[0]
        guard sha.count >= 7, sha.allSatisfy(\.isHexDigit) else { return nil }
        guard let date = gitDateFormatter.date(from: parts[2]) else { return nil }
        let message = parts[3]
        return GitCommit(
            sha: sha,
            authorName: parts[1],
            authoredAt: date,
            message: message,
            filesChanged: 0,
            insertions: 0,
            deletions: 0,
            inferredType: inferType(fromMessage: message)
        )
    }

    static func parseShortstat(_ line: String) -> (files: Int, insertions: Int, deletions: Int) {
        var files = 0, insertions = 0, deletions = 0
        let tokens = line.split(separator: " ").map(String.init)
        for (index, token) in tokens.enumerated() {
            guard let n = Int(token), index + 1 < tokens.count else { continue }
            let next = tokens[index + 1]
            if next.hasPrefix("file") { files = n }
            else if next.hasPrefix("insertion") { insertions = n }
            else if next.hasPrefix("deletion") { deletions = n }
        }
        return (files, insertions, deletions)
    }

    static func inferType(fromMessage message: String) -> String {
        let lower = message.lowercased()
        let conventional = ["feat", "fix", "refactor", "docs", "chore", "test", "style", "perf", "build", "ci", "wip"]

        // Conventional commit prefix: "type:" or "type(scope):"
        if let colon = lower.firstIndex(of: ":") {
            let beforeColon = lower[lower.startIndex..<colon]
            let type = String(beforeColon.prefix(while: { $0 != "(" }))
            if conventional.contains(type) { return type }
        }

        // Keyword fallback on the first word.
        let firstWord = String(lower.prefix(while: { $0.isLetter }))
        switch firstWord {
        case "add", "added", "adds", "create", "created", "implement", "implemented":
            return "feat"
        case "fix", "fixed", "fixes", "bug", "bugfix", "patch":
            return "fix"
        case "remove", "removed", "delete", "deleted", "drop":
            return "chore"
        case "update", "updated", "bump":
            return "chore"
        case "refactor", "refactored", "rework":
            return "refactor"
        case "doc", "docs", "document":
            return "docs"
        case "test", "tests":
            return "test"
        default:
            return "chore"
        }
    }

    // MARK: - Internals

    private static let gitDateFormatter: DateFormatter = {
        let f = DateFormatter()
        f.locale = Locale(identifier: "en_US_POSIX")
        f.dateFormat = "yyyy-MM-dd HH:mm:ss Z"
        return f
    }()

    private func run(_ arguments: [String]) throws -> String {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: gitPath)
        process.arguments = arguments
        let stdout = Pipe()
        process.standardOutput = stdout
        process.standardError = FileHandle.nullDevice
        try process.run()
        let data = stdout.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        return String(data: data, encoding: .utf8) ?? ""
    }
}
