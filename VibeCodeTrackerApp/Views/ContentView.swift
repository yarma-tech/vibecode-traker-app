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
    @Environment(SyncCenter.self) private var syncCenter
    @AppStorage(PreferenceKey.refreshFrequency) private var refreshFrequencyRaw = RefreshFrequency.hourly.rawValue
    @State private var selection: SidebarItem? = .dashboard
    @State private var hasScanned = false

    var body: some View {
        NavigationSplitView {
            ProjectListView(selection: $selection)
                .navigationSplitViewColumnWidth(min: 220, ideal: 240, max: 320)
        } detail: {
            DetailRouter(selection: selection)
        }
        .task {
            await scanOnce()
            await backgroundRefreshLoop()
        }
    }

    private func scanOnce() async {
        guard !hasScanned, !AppEnvironment.isRunningTests else { return }
        hasScanned = true
        await SyncCoordinator.fullSync(context: context, center: syncCenter)
    }

    /// Periodically re-runs the full sync per the user's refresh frequency.
    /// Stops for the "manual" setting or under tests.
    private func backgroundRefreshLoop() async {
        guard !AppEnvironment.isRunningTests else { return }
        while !Task.isCancelled {
            let frequency = RefreshFrequency(rawValue: refreshFrequencyRaw) ?? .hourly
            guard let interval = frequency.interval else { return }
            do {
                try await Task.sleep(for: .seconds(interval))
            } catch {
                return // cancelled
            }
            await SyncCoordinator.fullSync(context: context, center: syncCenter)
        }
    }
}

#Preview {
    ContentView()
        .modelContainer(for: Project.self, inMemory: true)
        .environment(SyncCenter())
}
