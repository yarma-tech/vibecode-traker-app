import XCTest
@testable import VibeCodeTrackerApp

/// Smoke test for Slice 1.
///
/// Its real job is to prove the test target links against the app target
/// (`@testable import` resolves) and that `xcodebuild test` runs end-to-end.
final class BootstrapTests: XCTestCase {
    func testContentViewCanBeInstantiated() {
        // Compiles only if the app module is importable and ContentView is visible.
        _ = ContentView()
    }
}
