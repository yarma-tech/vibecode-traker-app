import Foundation

/// Aggregated result of parsing one session's worth of JSONL lines.
struct ParsedSession: Equatable, Sendable {
    var sessionId: String
    var startedAt: Date
    var endedAt: Date
    var durationSeconds: Int
    var model: String
    var modelFamily: String
    var inputTokens: Int
    var outputTokens: Int
    var cacheReadTokens: Int
    var cacheCreationTokens: Int
    var messageCount: Int
    var firstUserPrompt: String?
    var cwd: String?
    var errorMessageCount: Int = 0
}

/// Parses Claude Code `.jsonl` transcripts into aggregated `ParsedSession` values.
///
/// Deliberately schema-tolerant: it decodes each line with `JSONSerialization`
/// into an untyped dictionary and reads only the keys it needs, so unexpected or
/// new line types and fields never cause a failure. Malformed lines are skipped.
final class JSONLParser {

    /// Maximum characters kept for the first-user-prompt preview.
    static let previewLength = 200

    // MARK: - Public API

    /// Parse raw JSONL text. Lines are grouped by `sessionId`; a line missing one
    /// falls back to `fallbackSessionId`. `defaultDate` is used for time bounds
    /// when a session has no parseable timestamps (typically the file's mtime).
    func parse(_ text: String, fallbackSessionId: String, defaultDate: Date) -> [ParsedSession] {
        var accumulators: [String: Accumulator] = [:]
        var order: [String] = []

        text.enumerateLines { line, _ in
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            guard !trimmed.isEmpty,
                  let data = trimmed.data(using: .utf8),
                  let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
            else { return }

            let sessionId = (object["sessionId"] as? String) ?? fallbackSessionId
            if accumulators[sessionId] == nil {
                accumulators[sessionId] = Accumulator(sessionId: sessionId)
                order.append(sessionId)
            }
            accumulators[sessionId]?.ingest(object)
        }

        return order.compactMap { accumulators[$0]?.finalize(defaultDate: defaultDate) }
    }

    /// Parse a `.jsonl` file. The filename (without extension) is the fallback
    /// session id; the file's modification date is the time fallback.
    func parseFile(at url: URL, defaultDate: Date) throws -> [ParsedSession] {
        let text = try String(contentsOf: url, encoding: .utf8)
        let fallback = url.deletingPathExtension().lastPathComponent
        return parse(text, fallbackSessionId: fallback, defaultDate: defaultDate)
    }

    // MARK: - Pure helpers (unit-tested)

    /// Coarse model family from a full model identifier.
    static func modelFamily(for model: String) -> String {
        let m = model.lowercased()
        if m.contains("opus") { return "Opus" }
        if m.contains("sonnet") { return "Sonnet" }
        if m.contains("haiku") { return "Haiku" }
        return "Unknown"
    }

    /// Parse an ISO-8601 timestamp, tolerating presence/absence of fractional seconds.
    static func parseTimestamp(_ string: String) -> Date? {
        if let d = isoWithFraction.date(from: string) { return d }
        return isoPlain.date(from: string)
    }

    /// Extract the first text from a message `content` value, which may be a plain
    /// string or an array of typed blocks. Returns nil if there is no text block
    /// (e.g. a tool-result-only user message).
    static func extractText(fromContent content: Any?) -> String? {
        if let string = content as? String {
            return cleanPreview(string)
        }
        if let blocks = content as? [[String: Any]] {
            for block in blocks where (block["type"] as? String) == "text" {
                if let text = block["text"] as? String {
                    return cleanPreview(text)
                }
            }
        }
        return nil
    }

    /// Error keywords used by the blocked-session heuristic (PRD §7).
    static let errorKeywords = ["error", "failed", "cannot", "unable to"]

    /// Concatenated text of a message `content` (string or text blocks).
    static func allText(fromContent content: Any?) -> String {
        if let string = content as? String { return string }
        if let blocks = content as? [[String: Any]] {
            return blocks.compactMap { block in
                (block["type"] as? String) == "text" ? block["text"] as? String : nil
            }.joined(separator: " ")
        }
        return ""
    }

    static func containsErrorKeyword(_ text: String) -> Bool {
        let lower = text.lowercased()
        return errorKeywords.contains { lower.contains($0) }
    }

    static func cleanPreview(_ text: String) -> String? {
        let collapsed = text
            .components(separatedBy: .whitespacesAndNewlines)
            .filter { !$0.isEmpty }
            .joined(separator: " ")
        guard !collapsed.isEmpty else { return nil }
        return String(collapsed.prefix(previewLength))
    }

    // MARK: - Internals

    private static let isoWithFraction: ISO8601DateFormatter = {
        let f = ISO8601DateFormatter()
        f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return f
    }()

    private static let isoPlain: ISO8601DateFormatter = {
        let f = ISO8601DateFormatter()
        f.formatOptions = [.withInternetDateTime]
        return f
    }()

    private static func intValue(_ any: Any?) -> Int {
        switch any {
        case let i as Int: return i
        case let n as NSNumber: return n.intValue
        case let d as Double: return Int(d)
        case let s as String: return Int(s) ?? 0
        default: return 0
        }
    }

    /// Accumulates lines belonging to a single session.
    private struct Accumulator {
        let sessionId: String
        var minTimestamp: Date?
        var maxTimestamp: Date?
        var inputTokens = 0
        var outputTokens = 0
        var cacheReadTokens = 0
        var cacheCreationTokens = 0
        /// Total tokens attributed to each model, to pick the primary one.
        var tokensByModel: [String: Int] = [:]
        var messageCount = 0
        var firstUserPrompt: String?
        var cwd: String?
        var errorMessageCount = 0

        mutating func ingest(_ object: [String: Any]) {
            let type = object["type"] as? String

            if cwd == nil, let c = object["cwd"] as? String, !c.isEmpty {
                cwd = c
            }

            if let ts = object["timestamp"] as? String, let date = JSONLParser.parseTimestamp(ts) {
                if let current = minTimestamp { minTimestamp = min(current, date) } else { minTimestamp = date }
                if let current = maxTimestamp { maxTimestamp = max(current, date) } else { maxTimestamp = date }
            }

            let message = object["message"] as? [String: Any]

            switch type {
            case "assistant":
                messageCount += 1
                if let message {
                    let usage = message["usage"] as? [String: Any]
                    let input = JSONLParser.intValue(usage?["input_tokens"])
                    let output = JSONLParser.intValue(usage?["output_tokens"])
                    let cacheRead = JSONLParser.intValue(usage?["cache_read_input_tokens"])
                    let cacheCreate = JSONLParser.intValue(usage?["cache_creation_input_tokens"])
                    inputTokens += input
                    outputTokens += output
                    cacheReadTokens += cacheRead
                    cacheCreationTokens += cacheCreate
                    if let model = message["model"] as? String, !model.isEmpty {
                        tokensByModel[model, default: 0] += input + output + cacheRead + cacheCreate
                    }
                    if JSONLParser.containsErrorKeyword(JSONLParser.allText(fromContent: message["content"])) {
                        errorMessageCount += 1
                    }
                }
            case "user":
                messageCount += 1
                if firstUserPrompt == nil, let message {
                    firstUserPrompt = JSONLParser.extractText(fromContent: message["content"])
                }
            default:
                break
            }
        }

        func finalize(defaultDate: Date) -> ParsedSession {
            let start = minTimestamp ?? defaultDate
            let end = maxTimestamp ?? start
            let duration = max(0, Int(end.timeIntervalSince(start)))
            let model = tokensByModel.max { $0.value < $1.value }?.key ?? ""
            return ParsedSession(
                sessionId: sessionId,
                startedAt: start,
                endedAt: end,
                durationSeconds: duration,
                model: model,
                modelFamily: JSONLParser.modelFamily(for: model),
                inputTokens: inputTokens,
                outputTokens: outputTokens,
                cacheReadTokens: cacheReadTokens,
                cacheCreationTokens: cacheCreationTokens,
                messageCount: messageCount,
                firstUserPrompt: firstUserPrompt,
                cwd: cwd,
                errorMessageCount: errorMessageCount
            )
        }
    }
}
