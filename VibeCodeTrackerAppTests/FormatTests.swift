import XCTest
@testable import VibeCodeTrackerApp

final class FormatTests: XCTestCase {
    func testTokens() {
        XCTAssertEqual(Format.tokens(0), "0")
        XCTAssertEqual(Format.tokens(950), "950")
        XCTAssertEqual(Format.tokens(1_200), "1.2k")
        XCTAssertEqual(Format.tokens(1_000), "1k")
        XCTAssertEqual(Format.tokens(2_400_000), "2.4M")
        XCTAssertEqual(Format.tokens(1_000_000), "1M")
    }

    func testDuration() {
        XCTAssertEqual(Format.duration(30), "30s")
        XCTAssertEqual(Format.duration(90), "1 min")
        XCTAssertEqual(Format.duration(2_520), "42 min")
        XCTAssertEqual(Format.duration(3_600), "1h 0m")
        XCTAssertEqual(Format.duration(3_900), "1h 5m")
    }
}
