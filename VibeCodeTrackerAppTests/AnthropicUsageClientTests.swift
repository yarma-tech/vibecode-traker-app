import XCTest
@testable import VibeCodeTrackerApp

/// URLProtocol that returns canned responses — no real network access.
final class MockURLProtocol: URLProtocol {
    static var handler: ((URLRequest) -> (HTTPURLResponse, Data))?

    override class func canInit(with request: URLRequest) -> Bool { true }
    override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        guard let handler = MockURLProtocol.handler else {
            client?.urlProtocol(self, didFailWithError: URLError(.badServerResponse))
            return
        }
        let (response, data) = handler(request)
        client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
        client?.urlProtocol(self, didLoad: data)
        client?.urlProtocolDidFinishLoading(self)
    }

    override func stopLoading() {}
}

final class AnthropicUsageClientTests: XCTestCase {
    override func tearDown() {
        MockURLProtocol.handler = nil
        super.tearDown()
    }

    private func makeClient() -> AnthropicUsageClient {
        let config = URLSessionConfiguration.ephemeral
        config.protocolClasses = [MockURLProtocol.self]
        return AnthropicUsageClient(session: URLSession(configuration: config))
    }

    private func http(_ url: URL, _ status: Int) -> HTTPURLResponse {
        HTTPURLResponse(url: url, statusCode: status, httpVersion: nil, headerFields: nil)
            ?? HTTPURLResponse()
    }

    func testSuccessfulParse() async throws {
        MockURLProtocol.handler = { request in
            XCTAssertEqual(request.value(forHTTPHeaderField: "x-api-key"), "sk-ant-test")
            XCTAssertEqual(request.value(forHTTPHeaderField: "anthropic-version"), "2023-06-01")
            let json = """
            {"data":[{"starting_at":"2026-05-01T00:00:00Z","ending_at":"2026-05-02T00:00:00Z","results":[
              {"model":"claude-sonnet-4-6","input_tokens":100,"output_tokens":200,"cache_read_input_tokens":300,"cache_creation_input_tokens":50}
            ]}]}
            """
            return (self.http(request.url ?? AnthropicUsageClient.defaultBaseURL, 200), Data(json.utf8))
        }

        let report = try await makeClient().fetchMessagesUsage(
            apiKey: "sk-ant-test",
            startingAt: Date(timeIntervalSince1970: 0),
            endingAt: Date(timeIntervalSince1970: 86_400)
        )
        XCTAssertEqual(report.buckets.count, 1)
        let bucket = try XCTUnwrap(report.buckets.first)
        XCTAssertEqual(bucket.model, "claude-sonnet-4-6")
        XCTAssertEqual(bucket.inputTokens, 100)
        XCTAssertEqual(bucket.outputTokens, 200)
        XCTAssertEqual(bucket.cacheReadTokens, 300)
        XCTAssertEqual(bucket.cacheCreationTokens, 50)
    }

    func testUnauthorized() async {
        MockURLProtocol.handler = { request in (self.http(request.url ?? AnthropicUsageClient.defaultBaseURL, 401), Data()) }
        await assertThrows(.unauthorized)
    }

    func testRateLimited() async {
        MockURLProtocol.handler = { request in (self.http(request.url ?? AnthropicUsageClient.defaultBaseURL, 429), Data()) }
        await assertThrows(.rateLimited)
    }

    func testServerError() async {
        MockURLProtocol.handler = { request in (self.http(request.url ?? AnthropicUsageClient.defaultBaseURL, 500), Data()) }
        await assertThrows(.http(500))
    }

    func testMissingKeyMakesNoRequest() async {
        MockURLProtocol.handler = { _ in XCTFail("Should not hit network without a key"); return (HTTPURLResponse(), Data()) }
        do {
            _ = try await makeClient().fetchMessagesUsage(apiKey: "", startingAt: Date(), endingAt: Date())
            XCTFail("Expected missingKey error")
        } catch {
            XCTAssertEqual(error as? AnthropicAPIError, .missingKey)
        }
    }

    private func assertThrows(_ expected: AnthropicAPIError, file: StaticString = #filePath, line: UInt = #line) async {
        do {
            _ = try await makeClient().fetchMessagesUsage(apiKey: "sk-ant-test", startingAt: Date(), endingAt: Date())
            XCTFail("Expected to throw \(expected)", file: file, line: line)
        } catch {
            XCTAssertEqual(error as? AnthropicAPIError, expected, file: file, line: line)
        }
    }
}
