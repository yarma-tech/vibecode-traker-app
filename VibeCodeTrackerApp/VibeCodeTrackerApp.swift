import SwiftUI
import SwiftData

/// Application entry point.
///
/// Owns the SwiftData `ModelContainer` and hosts the root `ContentView`.
@main
struct VibeCodeTrackerApp: App {
    let container: ModelContainer
    @State private var syncCenter = SyncCenter()

    init() {
        container = PersistenceController.makeContainer()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(syncCenter)
        }
        .defaultSize(width: 1100, height: 720)
        .modelContainer(container)
    }
}
