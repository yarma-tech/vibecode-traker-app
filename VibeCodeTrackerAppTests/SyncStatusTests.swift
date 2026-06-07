import XCTest
@testable import VibeCodeTrackerApp

final class SyncStatusTests: XCTestCase {
    func testLocalOnlyWhenDisabled() {
        XCTAssertEqual(SyncStatus.resolve(syncEnabled: false, cloudKitAvailable: true), .localOnly)
        XCTAssertEqual(SyncStatus.resolve(syncEnabled: false, cloudKitAvailable: false), .localOnly)
    }

    func testSetupRequiredWhenEnabledButUnavailable() {
        XCTAssertEqual(SyncStatus.resolve(syncEnabled: true, cloudKitAvailable: false), .cloudSetupRequired)
    }

    func testLabelsAndIconsArePresent() {
        XCTAssertFalse(SyncStatus.localOnly.label.isEmpty)
        XCTAssertFalse(SyncStatus.cloudSetupRequired.systemImage.isEmpty)
    }

    func testCloudKitInactiveInV1() {
        XCTAssertFalse(CloudKitSupport.isAvailable, "CloudKit must stay inert in V1")
    }
}
