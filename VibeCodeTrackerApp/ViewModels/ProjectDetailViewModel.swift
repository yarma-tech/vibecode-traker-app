import Foundation

/// Per-project KPIs (computed from that project's sessions only).
struct ProjectKPIs: Equatable {
    var sessionCount: Int = 0
    var totalTokens: Int = 0
    var totalCostUSD: Double? = nil
    var avgDurationSeconds: Int = 0

    static let empty = ProjectKPIs()
}

/// Pure per-project KPI math.
enum ProjectMetrics {
    static func kpis(stats: [SessionStat]) -> ProjectKPIs {
        guard !stats.isEmpty else { return .empty }
        var kpis = ProjectKPIs()
        kpis.sessionCount = stats.count
        kpis.totalTokens = stats.reduce(0) { $0 + $1.totalTokens }
        kpis.avgDurationSeconds = stats.reduce(0) { $0 + $1.durationSeconds } / stats.count
        let knownCosts = stats.compactMap(\.totalCostUSD)
        kpis.totalCostUSD = knownCosts.isEmpty ? nil : knownCosts.reduce(0, +)
        return kpis
    }
}

@MainActor
enum ProjectDetailViewModel {
    static func kpis(for project: Project) -> ProjectKPIs {
        ProjectMetrics.kpis(stats: GlobalDashboardViewModel.stats(from: project.sessions))
    }
}
