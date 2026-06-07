import Foundation

/// Per-project KPIs (computed from that project's sessions only).
struct ProjectKPIs: Equatable {
    var sessionCount: Int = 0
    var totalTokens: Int = 0
    var totalCostUSD: Double = 0
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
        kpis.totalCostUSD = stats.reduce(0) { $0 + $1.estimatedCostUSD }
        return kpis
    }
}

@MainActor
enum ProjectDetailViewModel {
    static func kpis(for project: Project) -> ProjectKPIs {
        ProjectMetrics.kpis(stats: GlobalDashboardViewModel.stats(from: project.sessions))
    }
}
