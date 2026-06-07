import XCTest
@testable import VibeCodeTrackerApp

final class ProjectFilterTests: XCTestCase {

    // MARK: - Worktree detection

    func testDetectIsWorktreeByPath() {
        XCTAssertTrue(Project.detectIsWorktree(
            path: "/Users/x/Developer/agent-qonto/.claude/worktrees/adoring-rubin-7708c3",
            claudeProjectHash: "anything"))
        XCTAssertFalse(Project.detectIsWorktree(
            path: "/Users/x/Developer/agent-qonto",
            claudeProjectHash: "-Users-x-Developer-agent-qonto"))
    }

    func testDetectIsWorktreeByHashFallback() {
        // Provisional projects (before path recovery) detect via the folder hash.
        XCTAssertTrue(Project.detectIsWorktree(
            path: "",
            claudeProjectHash: "-Users-x-Developer-agent-qonto--claude-worktrees-adoring-rubin-7708c3"))
    }

    // MARK: - Filter logic

    func testWorktreesHiddenByDefault() {
        XCTAssertFalse(ProjectFilter.matches(isWorktree: true, isOnGitHub: false, showWorktrees: false, repository: .all))
        XCTAssertTrue(ProjectFilter.matches(isWorktree: true, isOnGitHub: false, showWorktrees: true, repository: .all))
    }

    func testGitHubFilter() {
        XCTAssertTrue(ProjectFilter.matches(isWorktree: false, isOnGitHub: true, showWorktrees: false, repository: .gitHub))
        XCTAssertFalse(ProjectFilter.matches(isWorktree: false, isOnGitHub: false, showWorktrees: false, repository: .gitHub))
    }

    func testLocalOnlyFilter() {
        XCTAssertTrue(ProjectFilter.matches(isWorktree: false, isOnGitHub: false, showWorktrees: false, repository: .localOnly))
        XCTAssertFalse(ProjectFilter.matches(isWorktree: false, isOnGitHub: true, showWorktrees: false, repository: .localOnly))
    }

    func testWorktreeStillHiddenEvenWhenRepoMatches() {
        // A worktree must stay hidden regardless of the repository filter.
        XCTAssertFalse(ProjectFilter.matches(isWorktree: true, isOnGitHub: true, showWorktrees: false, repository: .gitHub))
    }
}
