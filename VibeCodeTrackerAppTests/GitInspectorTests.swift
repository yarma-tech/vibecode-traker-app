import XCTest
@testable import VibeCodeTrackerApp

final class GitInspectorTests: XCTestCase {

    // MARK: - Pure: type inference

    func testInferTypeFromConventionalPrefix() {
        XCTAssertEqual(GitInspector.inferType(fromMessage: "feat: add login"), "feat")
        XCTAssertEqual(GitInspector.inferType(fromMessage: "fix(parser): handle nulls"), "fix")
        XCTAssertEqual(GitInspector.inferType(fromMessage: "docs: update readme"), "docs")
    }

    func testInferTypeFromKeywords() {
        XCTAssertEqual(GitInspector.inferType(fromMessage: "Add new endpoint"), "feat")
        XCTAssertEqual(GitInspector.inferType(fromMessage: "Fixed a crash"), "fix")
        XCTAssertEqual(GitInspector.inferType(fromMessage: "Remove dead code"), "chore")
        XCTAssertEqual(GitInspector.inferType(fromMessage: "Refactor the store"), "refactor")
        XCTAssertEqual(GitInspector.inferType(fromMessage: "Something unrelated"), "chore")
    }

    // MARK: - Pure: log parsing

    func testParseLog() {
        let output = """
        abc1234abc1234abc1234abc1234abc1234abc1|Jane|2026-06-06 14:32:11 -0400|feat: add thing

         2 files changed, 10 insertions(+), 3 deletions(-)
        def5678def5678def5678def5678def5678def5|Joe|2026-06-05 09:00:00 -0400|fix: a bug

         1 file changed, 1 insertion(+)
        """
        let commits = GitInspector.parseLog(output)
        XCTAssertEqual(commits.count, 2)
        XCTAssertEqual(commits[0].authorName, "Jane")
        XCTAssertEqual(commits[0].message, "feat: add thing")
        XCTAssertEqual(commits[0].inferredType, "feat")
        XCTAssertEqual(commits[0].filesChanged, 2)
        XCTAssertEqual(commits[0].insertions, 10)
        XCTAssertEqual(commits[0].deletions, 3)
        XCTAssertEqual(commits[1].filesChanged, 1)
        XCTAssertEqual(commits[1].insertions, 1)
        XCTAssertEqual(commits[1].deletions, 0)
    }

    func testParseShortstat() {
        XCTAssertEqual(GitInspector.parseShortstat(" 3 files changed, 10 insertions(+), 2 deletions(-)").files, 3)
        XCTAssertEqual(GitInspector.parseShortstat(" 1 file changed, 5 insertions(+)").insertions, 5)
        XCTAssertEqual(GitInspector.parseShortstat(" 1 file changed, 2 deletions(-)").deletions, 2)
    }

    // MARK: - Pure: remotes

    func testParseRemotesDeduplicates() {
        let output = """
        origin\thttps://github.com/u/r.git (fetch)
        origin\thttps://github.com/u/r.git (push)
        upstream\tgit@gitlab.com:u/r.git (fetch)
        upstream\tgit@gitlab.com:u/r.git (push)
        """
        XCTAssertEqual(GitInspector.parseRemotes(output), ["https://github.com/u/r.git", "git@gitlab.com:u/r.git"])
    }

    func testParseRemotesEmpty() {
        XCTAssertEqual(GitInspector.parseRemotes(""), [])
    }

    func testIsGitHubURL() {
        XCTAssertTrue(GitInspector.isGitHubURL("https://github.com/u/r.git"))
        XCTAssertTrue(GitInspector.isGitHubURL("git@github.com:u/r.git"))
        XCTAssertFalse(GitInspector.isGitHubURL("git@gitlab.com:u/r.git"))
        XCTAssertFalse(GitInspector.isGitHubURL(""))
    }

    // MARK: - Integration against a real temp repo

    func testReadsCommitsFromTempRepo() throws {
        let repo = FileManager.default.temporaryDirectory.appending(path: "vct-repo-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: repo, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: repo) }

        try git(["init", "-b", "main"], in: repo)
        try git(["config", "user.email", "tester@example.com"], in: repo)
        try git(["config", "user.name", "Tester"], in: repo)
        try git(["config", "commit.gpgsign", "false"], in: repo)
        try "hello".write(to: repo.appending(path: "a.txt"), atomically: true, encoding: .utf8)
        try git(["add", "."], in: repo)
        try git(["commit", "-m", "feat: add a.txt"], in: repo)

        XCTAssertTrue(GitInspector.isGitRepository(repo.path))

        let commits = try GitInspector().commits(inRepository: repo.path)
        XCTAssertEqual(commits.count, 1)
        let commit = try XCTUnwrap(commits.first)
        XCTAssertEqual(commit.message, "feat: add a.txt")
        XCTAssertEqual(commit.inferredType, "feat")
        XCTAssertEqual(commit.authorName, "Tester")
        XCTAssertGreaterThanOrEqual(commit.filesChanged, 1)

        XCTAssertEqual(GitInspector().currentBranch(inRepository: repo.path), "main")
    }

    func testReadsRemotesFromTempRepo() throws {
        let repo = FileManager.default.temporaryDirectory.appending(path: "vct-repo-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: repo, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: repo) }

        try git(["init", "-b", "main"], in: repo)
        try git(["remote", "add", "origin", "https://github.com/test/repo.git"], in: repo)

        let remotes = GitInspector().remotes(inRepository: repo.path)
        XCTAssertEqual(remotes, ["https://github.com/test/repo.git"])
        XCTAssertTrue(remotes.contains(where: GitInspector.isGitHubURL))
    }

    func testIsGitRepositoryFalseForPlainDirectory() throws {
        let dir = FileManager.default.temporaryDirectory.appending(path: "vct-plain-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        XCTAssertFalse(GitInspector.isGitRepository(dir.path))
    }

    // MARK: - Helper

    private func git(_ args: [String], in directory: URL) throws {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        process.arguments = args
        process.currentDirectoryURL = directory
        process.standardOutput = FileHandle.nullDevice
        process.standardError = FileHandle.nullDevice
        try process.run()
        process.waitUntilExit()
        XCTAssertEqual(process.terminationStatus, 0, "git \(args.joined(separator: " ")) failed")
    }
}
