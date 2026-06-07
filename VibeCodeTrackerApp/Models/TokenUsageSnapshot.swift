import Foundation
import SwiftData

/// A snapshot of token usage + cost for a time bucket and model, fetched from the
/// Anthropic Console API (or computed locally as a fallback).
@Model
final class TokenUsageSnapshot {
    @Attribute(.unique) var id: UUID
    var fetchedAt: Date
    var periodStart: Date
    var periodEnd: Date
    var model: String
    var inputTokens: Int
    var outputTokens: Int
    var cacheReadTokens: Int
    var cacheCreationTokens: Int
    var costUSD: Double
    var source: String   // "local-jsonl" | "anthropic-api"

    init(
        id: UUID = UUID(),
        fetchedAt: Date = .now,
        periodStart: Date,
        periodEnd: Date,
        model: String,
        inputTokens: Int,
        outputTokens: Int,
        cacheReadTokens: Int,
        cacheCreationTokens: Int,
        costUSD: Double,
        source: String
    ) {
        self.id = id
        self.fetchedAt = fetchedAt
        self.periodStart = periodStart
        self.periodEnd = periodEnd
        self.model = model
        self.inputTokens = inputTokens
        self.outputTokens = outputTokens
        self.cacheReadTokens = cacheReadTokens
        self.cacheCreationTokens = cacheCreationTokens
        self.costUSD = costUSD
        self.source = source
    }
}
