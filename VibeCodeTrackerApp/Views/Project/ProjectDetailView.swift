import SwiftUI
import SwiftData
import AppKit

/// Level-2 view: a single project's header, local KPIs, stack, and sessions.
struct ProjectDetailView: View {
    let projectID: PersistentIdentifier
    @Environment(\.modelContext) private var context

    @AppStorage(PreferenceKey.detailSessionsExpanded) private var sessionsExpanded = true
    @AppStorage(PreferenceKey.detailCommitsExpanded) private var commitsExpanded = false
    @AppStorage(PreferenceKey.detailBacklogExpanded) private var backlogExpanded = false

    private let columns = [GridItem(.adaptive(minimum: 150), spacing: Spacing.md)]

    var body: some View {
        if let project = context.model(for: projectID) as? Project {
            content(for: project)
                .navigationTitle(project.name)
        } else {
            PlaceholderDetail(title: "Project not found", systemImage: "questionmark.folder", subtitle: nil)
        }
    }

    @ViewBuilder
    private func content(for project: Project) -> some View {
        let kpis = ProjectDetailViewModel.kpis(for: project)
        let sessions = project.sessions.sorted { $0.startedAt > $1.startedAt }
        let commits = project.commits.sorted { $0.authoredAt > $1.authoredAt }

        ScrollView {
            VStack(alignment: .leading, spacing: Spacing.lg) {
                header(for: project)

                LazyVGrid(columns: columns, spacing: Spacing.md) {
                    HeroKPI(title: "Sessions", value: "\(kpis.sessionCount)", systemImage: "bubble.left.and.bubble.right")
                    HeroKPI(title: "Tokens", value: Format.tokens(kpis.totalTokens), systemImage: "number")
                    HeroKPI(title: "Cost", value: String(format: "$%.2f", kpis.totalCostUSD), caption: "est.", systemImage: "dollarsign.circle")
                    HeroKPI(title: "Avg time", value: kpis.avgDurationSeconds > 0 ? Format.duration(kpis.avgDurationSeconds) : "—", systemImage: "clock")
                }

                // Commit activity (always shown)
                VStack(alignment: .leading, spacing: Spacing.sm) {
                    SectionHeader(title: "Commit activity — last 30 days")
                    if commits.isEmpty {
                        Text("No commits found (not a Git repo, or no recent activity).")
                            .font(.caption).foregroundStyle(.secondary)
                    } else {
                        CommitBarChartView(commitDates: commits.map(\.authoredAt))
                    }
                }

                // Recent sessions (collapsible, default open) — sessions lead this tracker
                VStack(alignment: .leading, spacing: Spacing.sm) {
                    SectionHeader(title: "Recent sessions", count: sessions.count, isExpanded: $sessionsExpanded)
                    if sessionsExpanded {
                        SessionsTableView(sessions: sessions)
                    }
                }

                // Recent commits (collapsible, default collapsed)
                VStack(alignment: .leading, spacing: Spacing.sm) {
                    SectionHeader(title: "Recent commits", count: commits.count, isExpanded: $commitsExpanded)
                    if commitsExpanded {
                        // CommitsListView renders its own muted empty line.
                        CommitsListView(commits: Array(commits.prefix(10)))
                    }
                }

                // Backlog (collapsible, default collapsed)
                VStack(alignment: .leading, spacing: Spacing.sm) {
                    SectionHeader(title: "Backlog", count: project.backlogItems.count, isExpanded: $backlogExpanded)
                    if backlogExpanded {
                        // BacklogView renders its own muted empty line.
                        BacklogView(items: project.backlogItems)
                    }
                }
            }
            .padding(Spacing.lg)
        }
    }

    @ViewBuilder
    private func header(for project: Project) -> some View {
        VStack(alignment: .leading, spacing: Spacing.xs) {
            HStack(alignment: .firstTextBaseline) {
                Text(project.name)
                    .font(.largeTitle.weight(.semibold))
                Spacer()
                Button {
                    openInFinder(project.path)
                } label: {
                    Label("Open in Finder", systemImage: "folder")
                }
                .controlSize(.small)
            }
            Text(project.path)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)
            Text("First seen \(project.firstSeenAt.formatted(date: .abbreviated, time: .omitted)) · Last activity \(Format.relative(project.lastActivityAt))")
                .font(.caption)
                .foregroundStyle(.secondary)
            DisclosureGroup {
                StackTagsView(stack: project.stack)
                    .padding(.top, Spacing.xs)
            } label: {
                Text("Stack")
                    .font(.subheadline.weight(.medium))
            }
            .padding(.top, Spacing.xs)
        }
    }

    private func openInFinder(_ path: String) {
        let url = URL(fileURLWithPath: path)
        guard FileManager.default.fileExists(atPath: path) else {
            Log.app.notice("Open in Finder: path does not exist: \(path, privacy: .public)")
            NSSound.beep()
            return
        }
        NSWorkspace.shared.activateFileViewerSelecting([url])
    }
}
