import Foundation
import SwiftData

/// A single Claude Code session, aggregated from one `.jsonl` transcript file.
@Model
final class Session {
    @Attribute(.unique) var sessionId: String
    var startedAt: Date
    var endedAt: Date?
    var durationSeconds: Int
    /// Full model identifier, e.g. "claude-sonnet-4-6-20250514".
    var model: String
    /// Coarse family: "Sonnet" | "Opus" | "Haiku" | "Unknown".
    var modelFamily: String
    var inputTokens: Int
    var outputTokens: Int
    var cacheReadTokens: Int
    var cacheCreationTokens: Int
    var totalCostUSD: Double?
    var topic: String?
    var category: String?
    /// "completed" | "inProgress" | "blocked" (refined in Slice 9).
    var status: String
    var filesModified: [String]
    var commitHashes: [String]
    var messageCount: Int
    /// Preview of the first real user prompt in the session.
    var firstUserPrompt: String?

    var project: Project?

    init(
        sessionId: String,
        startedAt: Date,
        endedAt: Date? = nil,
        durationSeconds: Int = 0,
        model: String = "",
        modelFamily: String = "Unknown",
        inputTokens: Int = 0,
        outputTokens: Int = 0,
        cacheReadTokens: Int = 0,
        cacheCreationTokens: Int = 0,
        totalCostUSD: Double? = nil,
        topic: String? = nil,
        category: String? = nil,
        status: String = SessionStatus.completed.rawValue,
        filesModified: [String] = [],
        commitHashes: [String] = [],
        messageCount: Int = 0,
        firstUserPrompt: String? = nil,
        project: Project? = nil
    ) {
        self.sessionId = sessionId
        self.startedAt = startedAt
        self.endedAt = endedAt
        self.durationSeconds = durationSeconds
        self.model = model
        self.modelFamily = modelFamily
        self.inputTokens = inputTokens
        self.outputTokens = outputTokens
        self.cacheReadTokens = cacheReadTokens
        self.cacheCreationTokens = cacheCreationTokens
        self.totalCostUSD = totalCostUSD
        self.topic = topic
        self.category = category
        self.status = status
        self.filesModified = filesModified
        self.commitHashes = commitHashes
        self.messageCount = messageCount
        self.firstUserPrompt = firstUserPrompt
        self.project = project
    }

    /// Sum of all token categories.
    var totalTokens: Int {
        inputTokens + outputTokens + cacheReadTokens + cacheCreationTokens
    }
}

/// Session lifecycle status. Stored as `rawValue` on `Session.status`.
enum SessionStatus: String, CaseIterable {
    case completed
    case inProgress
    case blocked
}
