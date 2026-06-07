import SwiftUI
import SwiftData

/// What the sidebar can select. Grows as new top-level destinations are added.
enum SidebarItem: Hashable {
    case dashboard
    case project(PersistentIdentifier)
    case settings
}

/// Root view: sidebar + detail. Kicks off an initial project scan on appear
/// (skipped under tests to keep them hermetic).
struct ContentView: View {
    @Environment(\.modelContext) private var context
    @State private var selection: SidebarItem? = .dashboard
    @State private var hasScanned = false

    var body: some View {
        NavigationSplitView {
            ProjectListView(selection: $selection)
                .navigationSplitViewColumnWidth(min: 220, ideal: 240, max: 320)
        } detail: {
            DetailRouter(selection: selection)
        }
        .task { await scanOnce() }
    }

    private func scanOnce() async {
        guard !hasScanned, !AppEnvironment.isRunningTests else { return }
        hasScanned = true
        await SyncCoordinator.fullSync(context: context)
    }
}

#Preview {
    ContentView()
        .modelContainer(for: Project.self, inMemory: true)
}
