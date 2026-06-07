import SwiftUI
import SwiftData

/// Level-1 view: headline KPIs + latest sessions across all projects.
struct GlobalDashboardView: View {
    @Environment(\.modelContext) private var context
    @Environment(SyncCenter.self) private var syncCenter
    @AppStorage(PreferenceKey.showWorktrees) private var showWorktrees = false
    @Query(sort: [SortDescriptor(\Session.startedAt, order: .reverse)])
    private var sessions: [Session]

    /// Sessions excluding worktrees (unless the user opted to show them).
    private var visibleSessions: [Session] {
        showWorktrees ? sessions : sessions.filter { !($0.project?.isWorktree ?? false) }
    }

    var body: some View {
        let kpis = GlobalDashboardViewModel.kpis(from: visibleSessions)

        ScrollView {
            VStack(alignment: .leading, spacing: Spacing.lg) {
                // Hero tier — the three decision metrics
                HStack(spacing: Spacing.md) {
                    HeroKPI(title: "Sessions", value: "\(kpis.sessionsThisWeek)", caption: "this week", systemImage: "bubble.left.and.bubble.right")
                    HeroKPI(title: "Cost", value: String(format: "$%.2f", kpis.costThisWeek), caption: "est. · this week", systemImage: "dollarsign.circle")
                    HeroKPI(title: "Blocked", value: "\(kpis.blockedSessions)", caption: "sessions", systemImage: "exclamationmark.triangle")
                }
                // Secondary tier — compact context metrics
                Card {
                    HStack(spacing: 0) {
                        SecondaryMetric(label: "Projects", value: "\(kpis.activeProjects)")
                        Divider().frame(height: 28)
                        SecondaryMetric(label: "Tokens", value: Format.tokens(kpis.tokensThisWeek))
                        Divider().frame(height: 28)
                        SecondaryMetric(label: "Avg/session", value: Format.tokens(kpis.avgTokensPerSession))
                        Divider().frame(height: 28)
                        SecondaryMetric(label: "Avg time", value: kpis.avgSessionDurationSeconds > 0 ? Format.duration(kpis.avgSessionDurationSeconds) : "—")
                        Divider().frame(height: 28)
                        SecondaryMetric(label: "Top model", value: kpis.topModelFamily)
                    }
                }

                content
            }
            .padding(Spacing.lg)
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
            VStack(alignment: .leading, spacing: Spacing.sm) {
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
