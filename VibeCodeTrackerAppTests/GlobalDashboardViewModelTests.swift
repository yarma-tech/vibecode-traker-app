import XCTest
@testable import VibeCodeTrackerApp

final class GlobalDashboardViewModelTests: XCTestCase {
    private let now = Date(timeIntervalSince1970: 1_750_000_000)
    private let day: TimeInterval = 86_400

    private func makeStats() -> [SessionStat] {
        [
            // within this week (< 7d)
            SessionStat(startedAt: now.addingTimeInterval(-1 * day), totalTokens: 1000, durationSeconds: 600, modelFamily: "Sonnet", status: "completed", projectKey: "projX"),
            SessionStat(startedAt: now.addingTimeInterval(-3 * day), totalTokens: 5000, durationSeconds: 1400, modelFamily: "Opus", status: "blocked", projectKey: "projY"),
            // within 30d, outside week
            SessionStat(startedAt: now.addingTimeInterval(-10 * day), totalTokens: 3000, durationSeconds: 1000, modelFamily: "Sonnet", status: "completed", projectKey: "projX"),
            // outside 30d
            SessionStat(startedAt: now.addingTimeInterval(-40 * day), totalTokens: 9999, durationSeconds: 9999, modelFamily: "Haiku", status: "completed", projectKey: "projZ"),
        ]
    }

    func testWeeklyMetrics() {
        let k = DashboardCalculator.kpis(stats: makeStats(), now: now)
        XCTAssertEqual(k.sessionsThisWeek, 2)
        XCTAssertEqual(k.tokensThisWeek, 6000)
    }

    func testThirtyDayMetrics() {
        let k = DashboardCalculator.kpis(stats: makeStats(), now: now)
        XCTAssertEqual(k.activeProjects, 2)             // projX, projY (projZ is >30d)
        XCTAssertEqual(k.avgTokensPerSession, 3000)     // (1000+5000+3000)/3
        XCTAssertEqual(k.avgSessionDurationSeconds, 1000) // (600+1400+1000)/3
    }

    func testTopModelAndShare() {
        let k = DashboardCalculator.kpis(stats: makeStats(), now: now)
        // 30d tokens: Sonnet 4000, Opus 5000 → Opus on top.
        XCTAssertEqual(k.topModelFamily, "Opus")
        XCTAssertEqual(k.topModelShare, 5000.0 / 9000.0, accuracy: 0.001)
    }

    func testBlockedCount() {
        let k = DashboardCalculator.kpis(stats: makeStats(), now: now)
        XCTAssertEqual(k.blockedSessions, 1)
    }

    func testEstimatedCostSummedForWeek() {
        var stats = makeStats()
        stats[0].estimatedCostUSD = 1.50   // in-week
        stats[1].estimatedCostUSD = 2.25   // in-week
        stats[2].estimatedCostUSD = 9.00   // outside week (within 30d) → excluded
        let k = DashboardCalculator.kpis(stats: stats, now: now)
        XCTAssertEqual(k.costThisWeek, 3.75, accuracy: 0.0001)
    }

    func testCostZeroWhenNoEstimates() {
        let k = DashboardCalculator.kpis(stats: makeStats(), now: now)
        XCTAssertEqual(k.costThisWeek, 0, accuracy: 0.0001)
    }

    func testEmptyIsAllZero() {
        let k = DashboardCalculator.kpis(stats: [], now: now)
        XCTAssertEqual(k, DashboardKPIs.empty)
        XCTAssertEqual(k.topModelFamily, "—")
    }
}
