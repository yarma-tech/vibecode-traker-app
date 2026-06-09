import XCTest
@testable import VibeCodeTrackerApp

final class BacklogRowDataTests: XCTestCase {

    private func make(title: String, isDone: Bool = false, line: Int = 0, priority: String? = nil) -> BacklogRowData {
        BacklogRowData(id: UUID(), title: title, isDone: isDone, sourceFile: "TODO.md", lineNumber: line, priority: priority)
    }

    func testPriorityRank() {
        XCTAssertEqual(make(title: "a", priority: "P0").priorityRank, 0)
        XCTAssertEqual(make(title: "a", priority: "P1").priorityRank, 1)
        XCTAssertEqual(make(title: "a", priority: "P2").priorityRank, 2)
        XCTAssertEqual(make(title: "a", priority: nil).priorityRank, 3)
        XCTAssertEqual(make(title: "a", priority: "weird").priorityRank, 3)
    }

    func testSortOpenBeforeDone() {
        let done = make(title: "done", isDone: true, line: 0)
        let open = make(title: "open", isDone: false, line: 99)
        let sorted = BacklogRowData.sorted([done, open])
        XCTAssertEqual(sorted.map(\.title), ["open", "done"])
    }

    func testSortByPriorityThenLine() {
        let p2 = make(title: "p2", line: 1, priority: "P2")
        let p0 = make(title: "p0", line: 2, priority: "P0")
        let p1a = make(title: "p1a", line: 5, priority: "P1")
        let p1b = make(title: "p1b", line: 3, priority: "P1")
        let sorted = BacklogRowData.sorted([p2, p0, p1a, p1b])
        XCTAssertEqual(sorted.map(\.title), ["p0", "p1b", "p1a", "p2"])
    }
}
