import XCTest
@testable import VibeCodeTrackerApp

final class CommitHeatmapTests: XCTestCase {
    private func utcCalendar() -> Calendar {
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = .gmt
        return cal
    }

    func testCellsCountCommitsPerDay() throws {
        let cal = utcCalendar()
        let now = Date(timeIntervalSince1970: 1_750_000_000)
        let today = cal.startOfDay(for: now)
        let yesterday = try XCTUnwrap(cal.date(byAdding: .day, value: -1, to: today))

        let cells = CommitHeatmap.cells(commitDates: [today, today, yesterday], now: now, weeks: 4, calendar: cal)

        let todayCell = cells.first { cal.isDate($0.date, inSameDayAs: today) }
        XCTAssertEqual(todayCell?.count, 2)
        let yesterdayCell = cells.first { cal.isDate($0.date, inSameDayAs: yesterday) }
        XCTAssertEqual(yesterdayCell?.count, 1)
    }

    func testNoFutureCells() {
        let cal = utcCalendar()
        let now = Date(timeIntervalSince1970: 1_750_000_000)
        let today = cal.startOfDay(for: now)
        let cells = CommitHeatmap.cells(commitDates: [], now: now, weeks: 4, calendar: cal)
        XCTAssertFalse(cells.isEmpty)
        XCTAssertNil(cells.first { $0.date > today }, "Heatmap must not include future days")
        XCTAssertTrue(cells.allSatisfy { $0.count == 0 })
    }
}
