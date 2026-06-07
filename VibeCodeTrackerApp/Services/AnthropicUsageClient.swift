import Foundation

/// One usage bucket from the Anthropic usage report.
struct UsageBucket: Equatable, Sendable {
    var startingAt: Date
    var endingAt: Date
    var model: String
    var inputTokens: Int
    var outputTokens: Int
    var cacheReadTokens: Int
    var cacheCreationTokens: Int
}

struct UsageReport: Equatable, Sendable {
    var buckets: [UsageBucket]
}

enum AnthropicAPIError: Error, Equatable {
    case missingKey
    case unauthorized          // 401 / 403
    case rateLimited           // 429
    case http(Int)
    case network(String)
    case decoding
}

/// Client for the Anthropic Console Admin usage API.
///
/// ONLY talks to `api.anthropic.com`. Never invoked without a real Admin key
/// supplied by the user (see UsageSyncService), so it makes no network calls by
/// default. The exact response schema is assumed/tolerant — see BLOCKERS.md.
final class AnthropicUsageClient {
    // Valid constant URL; force-unwrap is safe for this literal.
    static let defaultBaseURL = URL(string: "https://api.anthropic.com")!

    private let session: URLSession
    private let baseURL: URL

    init(session: URLSession = .shared, baseURL: URL = AnthropicUsageClient.defaultBaseURL) {
        self.session = session
        self.baseURL = baseURL
    }

    /// Fetches the messages usage report between two dates.
    func fetchMessagesUsage(
        apiKey: String,
        startingAt: Date,
        endingAt: Date,
        bucketWidth: String = "1d"
    ) async throws -> UsageReport {
        guard !apiKey.isEmpty else { throw AnthropicAPIError.missingKey }

        guard var components = URLComponents(url: baseURL.appendingPathComponent("/v1/organizations/usage_report/messages"), resolvingAgainstBaseURL: false) else {
            throw AnthropicAPIError.network("Invalid URL")
        }
        components.queryItems = [
            URLQueryItem(name: "starting_at", value: Self.iso.string(from: startingAt)),
            URLQueryItem(name: "ending_at", value: Self.iso.string(from: endingAt)),
            URLQueryItem(name: "bucket_width", value: bucketWidth)
        ]
        guard let url = components.url else { throw AnthropicAPIError.network("Invalid URL") }

        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        request.setValue(apiKey, forHTTPHeaderField: "x-api-key")
        request.setValue("2023-06-01", forHTTPHeaderField: "anthropic-version")

        let data: Data
        let response: URLResponse
        do {
            (data, response) = try await session.data(for: request)
        } catch {
            throw AnthropicAPIError.network(error.localizedDescription)
        }

        guard let http = response as? HTTPURLResponse else { throw AnthropicAPIError.network("No HTTP response") }
        switch http.statusCode {
        case 200...299:
            return try Self.parse(data)
        case 401, 403:
            throw AnthropicAPIError.unauthorized
        case 429:
            throw AnthropicAPIError.rateLimited
        default:
            throw AnthropicAPIError.http(http.statusCode)
        }
    }

    /// Lightweight connectivity/auth check used by Settings → Test connection.
    func testConnection(apiKey: String) async -> Result<Void, AnthropicAPIError> {
        do {
            _ = try await fetchMessagesUsage(
                apiKey: apiKey,
                startingAt: Date(timeIntervalSinceNow: -86_400),
                endingAt: Date()
            )
            return .success(())
        } catch let error as AnthropicAPIError {
            return .failure(error)
        } catch {
            return .failure(.network(error.localizedDescription))
        }
    }

    // MARK: - Parsing (tolerant)

    static func parse(_ data: Data) throws -> UsageReport {
        guard let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw AnthropicAPIError.decoding
        }
        let dataArray = object["data"] as? [[String: Any]] ?? []
        var buckets: [UsageBucket] = []
        for entry in dataArray {
            let start = (entry["starting_at"] as? String).flatMap(parseDate) ?? Date(timeIntervalSince1970: 0)
            let end = (entry["ending_at"] as? String).flatMap(parseDate) ?? start
            if let results = entry["results"] as? [[String: Any]], !results.isEmpty {
                for result in results {
                    buckets.append(makeBucket(result, start: start, end: end))
                }
            } else {
                buckets.append(makeBucket(entry, start: start, end: end))
            }
        }
        return UsageReport(buckets: buckets)
    }

    private static func makeBucket(_ dict: [String: Any], start: Date, end: Date) -> UsageBucket {
        UsageBucket(
            startingAt: start,
            endingAt: end,
            model: (dict["model"] as? String) ?? "unknown",
            inputTokens: intValue(dict["input_tokens"]),
            outputTokens: intValue(dict["output_tokens"]),
            cacheReadTokens: intValue(dict["cache_read_input_tokens"]),
            cacheCreationTokens: intValue(dict["cache_creation_input_tokens"])
        )
    }

    private static func parseDate(_ string: String) -> Date? {
        JSONLParser.parseTimestamp(string)
    }

    private static func intValue(_ any: Any?) -> Int {
        switch any {
        case let i as Int: return i
        case let n as NSNumber: return n.intValue
        case let d as Double: return Int(d)
        case let s as String: return Int(s) ?? 0
        default: return 0
        }
    }

    private static let iso: ISO8601DateFormatter = {
        let f = ISO8601DateFormatter()
        f.formatOptions = [.withInternetDateTime]
        return f
    }()
}
