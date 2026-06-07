import SwiftUI
import SwiftData

/// Level-1 view: headline KPIs + latest sessions across all projects.
struct GlobalDashboardView: View {
    @Environment(\.modelContext) private var context
    @Query(sort: [SortDescriptor(\Session.startedAt, order: .reverse)])
    private var sessions: [Session]

    @State private var isRefreshing = false

    private let columns = [GridItem(.adaptive(minimum: 150), spacing: 12)]

    var body: some View {
        let kpis = GlobalDashboardViewModel.kpis(from: sessions)

        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                LazyVGrid(columns: columns, spacing: 12) {
                    KPICard(title: "Projects", value: "\(kpis.activeProjects)", caption: "active", systemImage: "folder")
                    KPICard(title: "Sessions", value: "\(kpis.sessionsThisWeek)", caption: "this week", systemImage: "bubble.left.and.bubble.right")
                    KPICard(title: "Tokens", value: Format.tokens(kpis.tokensThisWeek), caption: "this week", systemImage: "number")
                    costCard(kpis)
                    KPICard(title: "Avg / session", value: Format.tokens(kpis.avgTokensPerSession), caption: "tokens · 30d", systemImage: "chart.bar")
                    KPICard(title: "Avg time", value: kpis.avgSessionDurationSeconds > 0 ? Format.duration(kpis.avgSessionDurationSeconds) : "—", caption: "30d", systemImage: "clock")
                    KPICard(title: "Top model", value: kpis.topModelFamily, caption: kpis.topModelShare > 0 ? "\(Int((kpis.topModelShare * 100).rounded())) %" : nil, systemImage: "cpu")
                    KPICard(title: "Blocked", value: "\(kpis.blockedSessions)", caption: "sessions", systemImage: "exclamationmark.triangle")
                }

                if !sessions.isEmpty {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Latest sessions across all projects")
                            .font(.headline)
                        LatestSessionsList(sessions: Array(sessions.prefix(10)))
                    }
                } else {
                    ContentUnavailableView(
                        "No sessions yet",
                        systemImage: "sparkles",
                        description: Text("Use Claude Code in a project, then press Refresh.")
                    )
                    .padding(.top, 40)
                }
            }
            .padding(20)
        }
        .navigationTitle("Global Dashboard")
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button {
                    Task { await refresh() }
                } label: {
                    Label("Refresh", systemImage: "arrow.clockwise")
                }
                .disabled(isRefreshing)
            }
        }
    }

    private func costCard(_ kpis: DashboardKPIs) -> some View {
        if let cost = kpis.costThisWeek {
            return KPICard(title: "Cost", value: String(format: "$%.2f", cost), caption: "this week", systemImage: "dollarsign.circle")
        } else {
            return KPICard(title: "Cost", value: "—", caption: "this week", systemImage: "dollarsign.circle",
                           tooltip: "Configure your Anthropic API key in Settings to see costs.")
        }
    }

    private func refresh() async {
        isRefreshing = true
        defer { isRefreshing = false }
        await SyncCoordinator.fullSync(context: context)
    }
}
