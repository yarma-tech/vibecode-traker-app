import XCTest
@testable import VibeCodeTrackerApp

final class SessionDisplayModeTests: XCTestCase {

    func testRawValueRoundTrip() {
        for mode in SessionDisplayMode.allCases {
            XCTAssertEqual(SessionDisplayMode(rawValue: mode.rawValue), mode)
        }
    }

    func testUnknownRawValueIsNil() {
        // The view maps nil to `.table`; here we just assert the raw lookup fails.
        XCTAssertNil(SessionDisplayMode(rawValue: "nope"))
    }

    func testHasExactlyTwoCases() {
        XCTAssertEqual(SessionDisplayMode.allCases.count, 2)
    }

    func testLabelsAndIconsPresent() {
        XCTAssertEqual(SessionDisplayMode.table.label, "Table")
        XCTAssertEqual(SessionDisplayMode.kanban.label, "Kanban")
        XCTAssertFalse(SessionDisplayMode.table.systemImage.isEmpty)
        XCTAssertFalse(SessionDisplayMode.kanban.systemImage.isEmpty)
    }
}
