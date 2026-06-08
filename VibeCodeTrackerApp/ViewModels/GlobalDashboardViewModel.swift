import Foundation

/// Plain, Sendable projection of a session used for KPI math (keeps the
/// calculator pure and trivially testable, independent of SwiftData).
struct SessionStat: Equatable, Sendable {
    let startedAt: Date
    let totalTokens: Int
    let durationSeconds: Int
    let modelFamily: String
    let status: String
    let projectKey: String?
    /// Estimated cost (USD) at published Anthropic list prices, derived from token counts.
    var estimatedCostUSD: Double = 0
}

/// The eight headline KPIs shown on the global dashboard.
struct DashboardKPIs: Equatable {
    var activeProjects: Int = 0
    var sessionsThisWeek: Int = 0
    var tokensThisWeek: Int = 0
    /// Estimated weekly cost at list prices (always available; 0 when no tokens).
    var costThisWeek: Double = 0
    var avgTokensPerSession: Int = 0
    var avgSessionDurationSeconds: Int = 0
    var topModelFamily: String = "—"
    /// Fraction (0...1) of 30-day tokens attributed to `topModelFamily`.
    var topModelShare: Double = 0
    var blockedSessions: Int = 0

    static let empty = DashboardKPIs()
}

/// Pure KPI computation over rolling windows relative to `now`.
///
/// "This week" = last 7 days; 30-day metrics = last 30 days. Rolling windows are
/// used (instead of calendar-week boundaries) for deterministic results and
/// timezone independence; see DECISIONS.md.
enum DashboardCalculator {
    static let weekSeconds: TimeInterval = 7 * 86_400
    static let monthSeconds: TimeInterval = 30 * 86_400

    static func kpis(stats: [SessionStat], now: Date) -> DashboardKPIs {
        let weekStart = now.addingTimeInterval(-weekSeconds)
        let monthStart = now.addingTimeInterval(-monthSeconds)

        let thisWeek = stats.filter { $0.startedAt >= weekStart }
        let last30 = stats.filter { $0.startedAt >= monthStart }

        var kpis = DashboardKPIs()
        kpis.sessionsThisWeek = thisWeek.count
        kpis.tokensThisWeek = thisWeek.reduce(0) { $0 + $1.totalTokens }

        let activeKeys = Set(last30.compactMap { $0.projectKey })
        kpis.activeProjects = activeKeys.count

        if !last30.isEmpty {
            let totalTokens = last30.reduce(0) { $0 + $1.totalTokens }
            let totalDuration = last30.reduce(0) { $0 + $1.durationSeconds }
            kpis.avgTokensPerSession = totalTokens / last30.count
            kpis.avgSessionDurationSeconds = totalDuration / last30.count

            var tokensByFamily: [String: Int] = [:]
            for stat in last30 where stat.totalTokens > 0 {
                tokensByFamily[stat.modelFamily, default: 0] += stat.totalTokens
            }
            if let top = tokensByFamily.max(by: { $0.value < $1.value }) {
                let familyTotal = tokensByFamily.values.reduce(0, +)
                kpis.topModelFamily = top.key
                kpis.topModelShare = familyTotal > 0 ? Double(top.value) / Double(familyTotal) : 0
            }
        }

        kpis.blockedSessions = stats.filter { $0.status == SessionStatus.blocked.rawValue }.count

        kpis.costThisWeek = thisWeek.reduce(0) { $0 + $1.estimatedCostUSD }

        return kpis
    }
}

/// View model bridging persisted `Session`s to the pure calculator.
@MainActor
enum GlobalDashboardViewModel {
    static func stats(from sessions: [Session]) -> [SessionStat] {
        sessions.map { session in
            SessionStat(
                startedAt: session.startedAt,
                totalTokens: session.totalTokens,
                durationSeconds: session.durationSeconds,
                modelFamily: session.modelFamily,
                status: session.status,
                projectKey: session.project?.claudeProjectHash,
                estimatedCostUSD: ModelPricing.cost(
                    family: session.modelFamily,
                    inputTokens: session.inputTokens,
                    outputTokens: session.outputTokens,
                    cacheReadTokens: session.cacheReadTokens,
                    cacheCreationTokens: session.cacheCreationTokens
                )
            )
        }
    }

    static func kpis(from sessions: [Session], now: Date = .now) -> DashboardKPIs {
        DashboardCalculator.kpis(stats: stats(from: sessions), now: now)
    }
}
