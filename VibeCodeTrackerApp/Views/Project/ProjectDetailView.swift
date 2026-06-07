import SwiftUI
import SwiftData
import AppKit

/// Level-2 view: a single project's header, local KPIs, stack, and sessions.
/// Commit timeline (Slice 6) and backlog (Slice 7) sections are added later.
struct ProjectDetailView: View {
    let projectID: PersistentIdentifier
    @Environment(\.modelContext) private var context

    private let columns = [GridItem(.adaptive(minimum: 150), spacing: 12)]

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

        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                header(for: project)

                LazyVGrid(columns: columns, spacing: 12) {
                    KPICard(title: "Sessions", value: "\(kpis.sessionCount)", systemImage: "bubble.left.and.bubble.right")
                    KPICard(title: "Tokens", value: Format.tokens(kpis.totalTokens), systemImage: "number")
                    if let cost = kpis.totalCostUSD {
                        KPICard(title: "Cost", value: String(format: "$%.2f", cost), systemImage: "dollarsign.circle")
                    } else {
                        KPICard(title: "Cost", value: "—", systemImage: "dollarsign.circle",
                                tooltip: "Configure your Anthropic API key in Settings to see costs.")
                    }
                    KPICard(title: "Avg time", value: kpis.avgDurationSeconds > 0 ? Format.duration(kpis.avgDurationSeconds) : "—", systemImage: "clock")
                }

                section("Commit activity — last 90 days") {
                    if project.commits.isEmpty {
                        Text("No commits found (not a Git repo, or no recent activity).")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    } else {
                        CommitHeatmapView(commitDates: project.commits.map(\.authoredAt))
                    }
                }

                section("Recent commits") {
                    CommitsListView(commits: Array(project.commits.sorted { $0.authoredAt > $1.authoredAt }.prefix(10)))
                }

                section("Stack") {
                    StackTagsView(stack: project.stack)
                }

                section("Recent sessions") {
                    SessionsListView(sessions: sessions)
                        .frame(minHeight: 120)
                }
            }
            .padding(20)
        }
    }

    @ViewBuilder
    private func header(for project: Project) -> some View {
        VStack(alignment: .leading, spacing: 6) {
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
        }
    }

    @ViewBuilder
    private func section<Content: View>(_ title: String, @ViewBuilder content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title).font(.headline)
            content()
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
