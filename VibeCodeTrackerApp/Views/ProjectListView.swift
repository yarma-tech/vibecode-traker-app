import SwiftUI
import SwiftData

/// Sidebar list: Global Dashboard, the detected projects (filtered), and Settings.
struct ProjectListView: View {
    @Binding var selection: SidebarItem?

    @AppStorage(PreferenceKey.syncViaICloud) private var syncViaICloud = false
    @AppStorage(PreferenceKey.showWorktrees) private var showWorktrees = false
    @AppStorage(PreferenceKey.repositoryFilter) private var repositoryFilterRaw = RepositoryFilter.all.rawValue

    @Query(sort: [SortDescriptor(\Project.lastActivityAt, order: .reverse)])
    private var projects: [Project]

    private var repositoryFilter: RepositoryFilter {
        RepositoryFilter(rawValue: repositoryFilterRaw) ?? .all
    }

    private var filteredProjects: [Project] {
        projects.filter {
            ProjectFilter.matches(isWorktree: $0.isWorktree, isOnGitHub: $0.isOnGitHub,
                                  showWorktrees: showWorktrees, repository: repositoryFilter)
        }
    }

    private var filterIsActive: Bool {
        showWorktrees || repositoryFilter != .all
    }

    var body: some View {
        List(selection: $selection) {
            Section {
                Label("Global Dashboard", systemImage: "square.grid.2x2")
                    .tag(SidebarItem.dashboard)
            }

            Section("Projects (\(filteredProjects.count))") {
                ForEach(filteredProjects) { project in
                    Label {
                        Text(project.name)
                    } icon: {
                        Image(systemName: project.isWorktree ? "arrow.triangle.branch" : (project.isOnGitHub ? "shippingbox" : "folder"))
                    }
                    .tag(SidebarItem.project(project.persistentModelID))
                }
            }

            Section {
                Label("Settings", systemImage: "gearshape")
                    .tag(SidebarItem.settings)
            }
        }
        .listStyle(.sidebar)
        .navigationTitle("Vibe Code Tracker")
        .toolbar {
            ToolbarItem {
                Menu {
                    Toggle("Show worktrees", isOn: $showWorktrees)
                    Picker("Repository", selection: $repositoryFilterRaw) {
                        ForEach(RepositoryFilter.allCases) { filter in
                            Text(filter.label).tag(filter.rawValue)
                        }
                    }
                } label: {
                    Label("Filter", systemImage: filterIsActive ? "line.3.horizontal.decrease.circle.fill" : "line.3.horizontal.decrease.circle")
                }
                .help("Filter projects")
            }
        }
        .safeAreaInset(edge: .bottom) {
            let status = SyncStatus.resolve(syncEnabled: syncViaICloud)
            HStack(spacing: 6) {
                Image(systemName: status.systemImage)
                Text(status.label)
                Spacer()
            }
            .font(.caption2)
            .foregroundStyle(.secondary)
            .padding(.horizontal, 14)
            .padding(.vertical, 8)
            .background(.bar)
        }
    }
}
