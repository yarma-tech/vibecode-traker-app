import XCTest
import SwiftUI
@testable import VibeCodeTrackerApp

final class CommitHeatmapTests: XCTestCase {

    // MARK: - Regression: chart domain must be a valid (ascending) range.

    func testWeekdayDomainIsAscending() {
        // A descending ClosedRange traps at runtime — this guards against the
        // reversed `6.5...(-0.5)` that crashed CommitHeatmapView.body.
        XCTAssertLessThan(CommitHeatmap.weekdayDomain.lowerBound, CommitHeatmap.weekdayDomain.upperBound)
    }

    func testRowForWeekdayPutsSundayOnTopAndStaysInDomain() {
        XCTAssertEqual(CommitHeatmap.row(forWeekday: 0), 6) // Sunday at top row
        XCTAssertEqual(CommitHeatmap.row(forWeekday: 6), 0)
        for weekday in 0...6 {
            XCTAssertTrue(CommitHeatmap.weekdayDomain.contains(Double(CommitHeatmap.row(forWeekday: weekday))))
        }
    }

    @MainActor
    func testHeatmapViewBodyRenders() throws {
        // Forces the SwiftUI body (and the chart domain) to evaluate — would have
        // trapped before the fix. Renders to an image headlessly.
        let view = CommitHeatmapView(commitDates: [Date(), Date(timeIntervalSinceNow: -86_400)], weeks: 13)
        let renderer = ImageRenderer(content: view.frame(width: 320, height: 110))
        XCTAssertNotNil(renderer.nsImage, "Heatmap must render without trapping")
    }
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
