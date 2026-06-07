import SwiftUI

/// Application entry point.
///
/// Slice 1 keeps this intentionally minimal: a single window hosting the
/// placeholder `ContentView`. The SwiftData `ModelContainer`, sidebar
/// navigation, and background scanning are wired in later slices.
@main
struct VibeCodeTrackerApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
        .defaultSize(width: 1100, height: 720)
    }
}
