import Foundation
import SwiftData

struct UsageSyncSummary: Equatable, Sendable {
    var snapshotsWritten = 0
    var sessionsPriced = 0
    var skippedNoKey = false
}

/// Fetches usage from the Anthropic API (when a key is configured), stores
/// `TokenUsageSnapshot`s, and prices local sessions so cost KPIs light up.
///
/// If no Admin key is in the Keychain, this is a no-op — the app stays in
/// tokens-only mode and never makes a network call.
final class UsageSyncService {
    private let secretStore: SecretStoring
    private let client: AnthropicUsageClient

    init(secretStore: SecretStoring = KeychainStore(), client: AnthropicUsageClient = AnthropicUsageClient()) {
        self.secretStore = secretStore
        self.client = client
    }

    @MainActor
    @discardableResult
    func sync(into context: ModelContext, now: Date = .now) async throws -> UsageSyncSummary {
        guard let apiKey = try? secretStore.get(), !apiKey.isEmpty else {
            return UsageSyncSummary(skippedNoKey: true)
        }

        let start = now.addingTimeInterval(-30 * 86_400)
        let report = try await client.fetchMessagesUsage(apiKey: apiKey, startingAt: start, endingAt: now)

        var summary = UsageSyncSummary()
        summary.snapshotsWritten = try storeSnapshots(report, into: context, now: now)
        summary.sessionsPriced = try priceSessions(in: context)
        Log.network.info("Usage sync: \(summary.snapshotsWritten) snapshots, \(summary.sessionsPriced) sessions priced.")
        return summary
    }

    // MARK: - Snapshots

    @MainActor
    private func storeSnapshots(_ report: UsageReport, into context: ModelContext, now: Date) throws -> Int {
        let existing = try context.fetch(FetchDescriptor<TokenUsageSnapshot>())
        var byKey: [String: TokenUsageSnapshot] = [:]
        for snapshot in existing {
            byKey[Self.snapshotKey(periodStart: snapshot.periodStart, model: snapshot.model, source: snapshot.source)] = snapshot
        }

        var written = 0
        for bucket in report.buckets {
            let family = JSONLParser.modelFamily(for: bucket.model)
            let cost = ModelPricing.cost(
                family: family,
                inputTokens: bucket.inputTokens,
                outputTokens: bucket.outputTokens,
                cacheReadTokens: bucket.cacheReadTokens,
                cacheCreationTokens: bucket.cacheCreationTokens
            )
            let key = Self.snapshotKey(periodStart: bucket.startingAt, model: bucket.model, source: "anthropic-api")
            if let snapshot = byKey[key] {
                snapshot.inputTokens = bucket.inputTokens
                snapshot.outputTokens = bucket.outputTokens
                snapshot.cacheReadTokens = bucket.cacheReadTokens
                snapshot.cacheCreationTokens = bucket.cacheCreationTokens
                snapshot.costUSD = cost
                snapshot.fetchedAt = now
            } else {
                let snapshot = TokenUsageSnapshot(
                    fetchedAt: now,
                    periodStart: bucket.startingAt,
                    periodEnd: bucket.endingAt,
                    model: bucket.model,
                    inputTokens: bucket.inputTokens,
                    outputTokens: bucket.outputTokens,
                    cacheReadTokens: bucket.cacheReadTokens,
                    cacheCreationTokens: bucket.cacheCreationTokens,
                    costUSD: cost,
                    source: "anthropic-api"
                )
                context.insert(snapshot)
                byKey[key] = snapshot
            }
            written += 1
        }
        if context.hasChanges { try context.save() }
        return written
    }

    // MARK: - Per-session cost reconciliation

    @MainActor
    private func priceSessions(in context: ModelContext) throws -> Int {
        let sessions = try context.fetch(FetchDescriptor<Session>())
        for session in sessions {
            session.totalCostUSD = ModelPricing.cost(
                family: session.modelFamily,
                inputTokens: session.inputTokens,
                outputTokens: session.outputTokens,
                cacheReadTokens: session.cacheReadTokens,
                cacheCreationTokens: session.cacheCreationTokens
            )
        }
        if context.hasChanges { try context.save() }
        return sessions.count
    }

    static func snapshotKey(periodStart: Date, model: String, source: String) -> String {
        "\(Int(periodStart.timeIntervalSince1970))|\(model)|\(source)"
    }
}
