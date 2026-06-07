import XCTest
@testable import VibeCodeTrackerApp

final class CommitBarsTests: XCTestCase {
    private func utcCalendar() -> Calendar {
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = .gmt
        return cal
    }
    private let now = Date(timeIntervalSince1970: 1_750_000_000)

    func testSeriesHasOneBarPerDayInWindow() {
        let bars = CommitBars.series(commitDates: [], now: now, days: 30, calendar: utcCalendar())
        XCTAssertEqual(bars.count, 30)
        XCTAssertTrue(bars.allSatisfy { $0.count == 0 }, "empty input → all zero-filled days")
    }

    func testCountsCommitsPerDay() throws {
        let cal = utcCalendar()
        let today = cal.startOfDay(for: now)
        let yesterday = try XCTUnwrap(cal.date(byAdding: .day, value: -1, to: today))
        let bars = CommitBars.series(commitDates: [today, today, yesterday], now: now, days: 30, calendar: cal)
        XCTAssertEqual(bars.first { cal.isDate($0.day, inSameDayAs: today) }?.count, 2)
        XCTAssertEqual(bars.first { cal.isDate($0.day, inSameDayAs: yesterday) }?.count, 1)
    }

    func testNoFutureDaysAndIsAscending() {
        let cal = utcCalendar()
        let today = cal.startOfDay(for: now)
        let bars = CommitBars.series(commitDates: [], now: now, days: 30, calendar: cal)
        XCTAssertNil(bars.first { $0.day > today }, "must not include future days")
        XCTAssertEqual(bars.map(\.day), bars.map(\.day).sorted(), "days ascending")
    }

    func testCommitsOutsideWindowAreIgnored() throws {
        let cal = utcCalendar()
        let today = cal.startOfDay(for: now)
        let old = try XCTUnwrap(cal.date(byAdding: .day, value: -90, to: today))
        let bars = CommitBars.series(commitDates: [old], now: now, days: 30, calendar: cal)
        XCTAssertEqual(bars.reduce(0) { $0 + $1.count }, 0, "commits older than the window contribute nothing")
    }
}
