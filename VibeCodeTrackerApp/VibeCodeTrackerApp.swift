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
        // A second copy sharing this bundle id would corrupt the shared SwiftData
        // store (see SingleInstanceGuard). Hand off to the existing instance and
        // exit before opening the container.
        if SingleInstanceGuard.anotherInstanceIsRunning() {
            exit(0)
        }
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
