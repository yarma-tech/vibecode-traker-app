import SwiftUI
import SwiftData

/// Sidebar list: Global Dashboard, the detected projects (most recent first),
/// and Settings.
struct ProjectListView: View {
    @Binding var selection: SidebarItem?

    @AppStorage(PreferenceKey.syncViaICloud) private var syncViaICloud = false

    @Query(sort: [SortDescriptor(\Project.lastActivityAt, order: .reverse)])
    private var projects: [Project]

    var body: some View {
        List(selection: $selection) {
            Section {
                Label("Global Dashboard", systemImage: "square.grid.2x2")
                    .tag(SidebarItem.dashboard)
            }

            Section("Projects") {
                ForEach(projects) { project in
                    Label {
                        Text(project.name)
                    } icon: {
                        Image(systemName: "folder")
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
