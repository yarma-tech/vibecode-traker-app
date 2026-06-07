import XCTest
@testable import VibeCodeTrackerApp

final class ModelPricingTests: XCTestCase {
    func testSonnetCost() {
        // 1M input @ $3, 1M output @ $15 → $18.
        let cost = ModelPricing.cost(family: "Sonnet", inputTokens: 1_000_000, outputTokens: 1_000_000, cacheReadTokens: 0, cacheCreationTokens: 0)
        XCTAssertEqual(cost, 18.0, accuracy: 0.0001)
    }

    func testCacheTokensPriced() {
        // 1M cache read @ $0.30 for Sonnet.
        let cost = ModelPricing.cost(family: "Sonnet", inputTokens: 0, outputTokens: 0, cacheReadTokens: 1_000_000, cacheCreationTokens: 0)
        XCTAssertEqual(cost, 0.30, accuracy: 0.0001)
    }

    func testUnknownFamilyIsFree() {
        XCTAssertEqual(ModelPricing.cost(family: "Unknown", inputTokens: 1_000_000, outputTokens: 1_000_000, cacheReadTokens: 0, cacheCreationTokens: 0), 0)
        XCTAssertFalse(ModelPricing.isKnown(family: "Unknown"))
        XCTAssertTrue(ModelPricing.isKnown(family: "Opus"))
    }
}
