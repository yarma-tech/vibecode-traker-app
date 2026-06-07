import Foundation

/// Per-token USD prices for a model family.
struct ModelPrice: Equatable {
    let input: Double
    let output: Double
    let cacheWrite: Double
    let cacheRead: Double
}

/// Approximate public Anthropic pricing used to derive USD from token counts.
///
/// NOTE: these are published list prices per million tokens (as of the build
/// date) and are an ESTIMATE. The authoritative figure is whatever the Anthropic
/// Console reports; see BLOCKERS.md. Prices are easy to update here in one place.
enum ModelPricing {
    /// Prices in USD per million tokens, by family.
    private static let perMillion: [String: ModelPrice] = [
        "Opus":   ModelPrice(input: 15.0, output: 75.0, cacheWrite: 18.75, cacheRead: 1.50),
        "Sonnet": ModelPrice(input: 3.0,  output: 15.0, cacheWrite: 3.75,  cacheRead: 0.30),
        "Haiku":  ModelPrice(input: 0.80, output: 4.0,  cacheWrite: 1.0,   cacheRead: 0.08)
    ]

    /// Price per single token for a family (zero for unknown families).
    static func price(family: String) -> ModelPrice {
        guard let perM = perMillion[family] else {
            return ModelPrice(input: 0, output: 0, cacheWrite: 0, cacheRead: 0)
        }
        return ModelPrice(
            input: perM.input / 1_000_000,
            output: perM.output / 1_000_000,
            cacheWrite: perM.cacheWrite / 1_000_000,
            cacheRead: perM.cacheRead / 1_000_000
        )
    }

    static func cost(family: String, inputTokens: Int, outputTokens: Int, cacheReadTokens: Int, cacheCreationTokens: Int) -> Double {
        let p = price(family: family)
        return Double(inputTokens) * p.input
            + Double(outputTokens) * p.output
            + Double(cacheReadTokens) * p.cacheRead
            + Double(cacheCreationTokens) * p.cacheWrite
    }

    static func isKnown(family: String) -> Bool { perMillion[family] != nil }
}
