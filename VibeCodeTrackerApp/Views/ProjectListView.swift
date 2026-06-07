import SwiftUI
import SwiftData

/// Sidebar list: Global Dashboard, the detected projects (most recent first),
/// and Settings.
struct ProjectListView: View {
    @Binding var selection: SidebarItem?

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
    }
}
