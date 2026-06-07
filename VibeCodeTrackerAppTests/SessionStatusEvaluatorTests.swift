import XCTest
@testable import VibeCodeTrackerApp

final class SessionStatusEvaluatorTests: XCTestCase {
    private let now = Date(timeIntervalSince1970: 1_750_000_000)

    func testInProgressWhenRecentlyModified() {
        let status = SessionStatusEvaluator.status(
            errorMessageCount: 10,                       // would be blocked, but...
            fileModifiedAt: now.addingTimeInterval(-60), // ...modified 1 min ago wins
            hasAssociatedCommit: false,
            now: now
        )
        XCTAssertEqual(status, .inProgress)
    }

    func testBlockedWhenManyErrorsAndNoCommit() {
        let status = SessionStatusEvaluator.status(
            errorMessageCount: 3,
            fileModifiedAt: now.addingTimeInterval(-3600), // 1h ago, not in progress
            hasAssociatedCommit: false,
            now: now
        )
        XCTAssertEqual(status, .blocked)
    }

    func testNotBlockedWhenCommitAssociated() {
        let status = SessionStatusEvaluator.status(
            errorMessageCount: 5,
            fileModifiedAt: now.addingTimeInterval(-3600),
            hasAssociatedCommit: true,
            now: now
        )
        XCTAssertEqual(status, .completed)
    }

    func testCompletedWhenFewErrors() {
        let status = SessionStatusEvaluator.status(
            errorMessageCount: 1,
            fileModifiedAt: now.addingTimeInterval(-7200),
            hasAssociatedCommit: false,
            now: now
        )
        XCTAssertEqual(status, .completed)
    }

    func testHasCommitWindow() {
        let start = now
        let end = now.addingTimeInterval(600)
        XCTAssertTrue(SessionStatusEvaluator.hasCommit(commitDates: [now.addingTimeInterval(300)], startedAt: start, endedAt: end))
        // commit within the 5-minute grace after end
        XCTAssertTrue(SessionStatusEvaluator.hasCommit(commitDates: [end.addingTimeInterval(120)], startedAt: start, endedAt: end))
        // commit before the session
        XCTAssertFalse(SessionStatusEvaluator.hasCommit(commitDates: [now.addingTimeInterval(-60)], startedAt: start, endedAt: end))
    }
}
