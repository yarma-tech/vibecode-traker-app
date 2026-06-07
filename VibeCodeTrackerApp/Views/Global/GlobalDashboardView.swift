import SwiftUI
import SwiftData

/// Level-1 view: headline KPIs + latest sessions across all projects.
struct GlobalDashboardView: View {
    @Environment(\.modelContext) private var context
    @Environment(SyncCenter.self) private var syncCenter
    @AppStorage(PreferenceKey.showWorktrees) private var showWorktrees = false
    @Query(sort: [SortDescriptor(\Session.startedAt, order: .reverse)])
    private var sessions: [Session]

    private let columns = [GridItem(.adaptive(minimum: 150), spacing: 12)]

    /// Sessions excluding worktrees (unless the user opted to show them).
    private var visibleSessions: [Session] {
        showWorktrees ? sessions : sessions.filter { !($0.project?.isWorktree ?? false) }
    }

    var body: some View {
        let kpis = GlobalDashboardViewModel.kpis(from: visibleSessions)

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

                content
            }
            .padding(20)
            .animation(.default, value: visibleSessions.isEmpty)
        }
        .navigationTitle("Global Dashboard")
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                RefreshButton(isSyncing: syncCenter.isSyncing) {
                    Task { await SyncCoordinator.fullSync(context: context, center: syncCenter) }
                }
            }
        }
    }

    @ViewBuilder
    private var content: some View {
        if !visibleSessions.isEmpty {
            VStack(alignment: .leading, spacing: 8) {
                Text("Latest sessions across all projects")
                    .font(.headline)
                LatestSessionsList(sessions: Array(visibleSessions.prefix(10)))
            }
        } else if syncCenter.isSyncing {
            VStack(spacing: 12) {
                ProgressView()
                Text("Scanning your Claude Code projects…")
                    .foregroundStyle(.secondary)
            }
            .frame(maxWidth: .infinity)
            .padding(.top, 60)
        } else {
            ContentUnavailableView {
                Label("No sessions yet", systemImage: "sparkles")
            } description: {
                Text("Use Claude Code in a project, then press Refresh. Vibe Code Tracker reads ~/.claude/projects locally.")
            } actions: {
                Button("Refresh") { Task { await SyncCoordinator.fullSync(context: context, center: syncCenter) } }
            }
            .padding(.top, 40)
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
}

/// Toolbar refresh button that spins while a sync is in progress.
private struct RefreshButton: View {
    let isSyncing: Bool
    let action: () -> Void
    @State private var angle = 0.0

    var body: some View {
        Button(action: action) {
            Label("Refresh", systemImage: "arrow.clockwise")
                .rotationEffect(.degrees(angle))
        }
        .disabled(isSyncing)
        .onChange(of: isSyncing) { _, syncing in
            if syncing {
                withAnimation(.linear(duration: 1).repeatForever(autoreverses: false)) { angle = 360 }
            } else {
                withAnimation(.default) { angle = 0 }
            }
        }
    }
}
