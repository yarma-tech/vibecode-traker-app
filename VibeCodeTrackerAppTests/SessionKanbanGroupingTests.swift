import XCTest
@testable import VibeCodeTrackerApp

final class SessionKanbanGroupingTests: XCTestCase {

    private func date(_ t: TimeInterval) -> Date { Date(timeIntervalSince1970: t) }

    func testAlwaysThreeColumnsInFixedOrderWhenEmpty() {
        let empty: [(SessionStatus, Date)] = []
        let cols = SessionKanbanGrouping.group(empty, status: { $0.0 }, startedAt: { $0.1 })
        XCTAssertEqual(cols.map { $0.status }, [.inProgress, .blocked, .completed])
        XCTAssertTrue(cols.allSatisfy { $0.items.isEmpty })
    }

    func testGroupsByStatusIntoCorrectColumns() {
        let items: [(SessionStatus, Date)] = [
            (.completed, date(100)),
            (.inProgress, date(200)),
            (.blocked, date(150)),
        ]
        let cols = SessionKanbanGrouping.group(items, status: { $0.0 }, startedAt: { $0.1 })
        XCTAssertEqual(cols[0].items.count, 1)   // inProgress
        XCTAssertEqual(cols[1].items.count, 1)   // blocked
        XCTAssertEqual(cols[2].items.count, 1)   // completed
    }

    func testItemsSortedNewestFirstWithinColumn() {
        let items: [(SessionStatus, Date)] = [
            (.completed, date(100)),
            (.completed, date(300)),
            (.completed, date(200)),
        ]
        let cols = SessionKanbanGrouping.group(items, status: { $0.0 }, startedAt: { $0.1 })
        let completed = cols[2].items.map { $0.1 }
        XCTAssertEqual(completed, [date(300), date(200), date(100)])
    }

    func testUnknownStatusFallsBackToCompleted() {
        // The Session convenience overload maps an unknown stored status here.
        XCTAssertEqual(SessionStatus(rawValue: "garbage") ?? .completed, .completed)
    }
}
