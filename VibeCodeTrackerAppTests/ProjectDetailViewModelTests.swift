import XCTest
@testable import VibeCodeTrackerApp

final class ProjectDetailViewModelTests: XCTestCase {
    private let t0 = Date(timeIntervalSince1970: 1_750_000_000)

    private func stat(tokens: Int, duration: Int, cost: Double? = nil) -> SessionStat {
        SessionStat(startedAt: t0, totalTokens: tokens, durationSeconds: duration,
                    modelFamily: "Sonnet", status: "completed", projectKey: "p", totalCostUSD: cost)
    }

    func testEmpty() {
        XCTAssertEqual(ProjectMetrics.kpis(stats: []), ProjectKPIs.empty)
    }

    func testAggregates() {
        let kpis = ProjectMetrics.kpis(stats: [
            stat(tokens: 1000, duration: 600),
            stat(tokens: 3000, duration: 1200),
        ])
        XCTAssertEqual(kpis.sessionCount, 2)
        XCTAssertEqual(kpis.totalTokens, 4000)
        XCTAssertEqual(kpis.avgDurationSeconds, 900)
        XCTAssertNil(kpis.totalCostUSD)
    }

    func testCostSummedWhenPresent() {
        let kpis = ProjectMetrics.kpis(stats: [
            stat(tokens: 10, duration: 1, cost: 0.50),
            stat(tokens: 20, duration: 2, cost: 1.00),
            stat(tokens: 30, duration: 3),
        ])
        XCTAssertEqual(try XCTUnwrap(kpis.totalCostUSD), 1.50, accuracy: 0.0001)
    }
}
