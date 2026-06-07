import XCTest
@testable import VibeCodeTrackerApp

final class JSONLParserTests: XCTestCase {
    private let parser = JSONLParser()
    private let epoch = Date(timeIntervalSince1970: 0)

    // MARK: - Fixture aggregation

    func testParsesBasicFixtureAggregates() throws {
        let text = try TestSupport.fixtureText("session-basic.jsonl")
        let sessions = parser.parse(text, fallbackSessionId: "fallback", defaultDate: epoch)

        XCTAssertEqual(sessions.count, 1)
        let s = try XCTUnwrap(sessions.first)

        XCTAssertEqual(s.sessionId, "11111111-1111-1111-1111-111111111111")
        XCTAssertEqual(s.inputTokens, 15)
        XCTAssertEqual(s.outputTokens, 300)
        XCTAssertEqual(s.cacheReadTokens, 3000)
        XCTAssertEqual(s.cacheCreationTokens, 50)
        XCTAssertEqual(s.model, "claude-sonnet-4-6-20250514")
        XCTAssertEqual(s.modelFamily, "Sonnet")
        XCTAssertEqual(s.durationSeconds, 300) // 10:00:00 → 10:05:00
        XCTAssertEqual(s.messageCount, 4)      // 2 user + 2 assistant
        XCTAssertEqual(s.firstUserPrompt, "Add a login screen please")
        XCTAssertEqual(s.cwd, "/Users/demo/Developer/login-app")
    }

    func testFirstUserPromptIgnoresToolResultMessages() throws {
        // The fixture's second user message is a tool_result; the preview must be
        // the first *text* user prompt, not the tool result.
        let text = try TestSupport.fixtureText("session-basic.jsonl")
        let s = try XCTUnwrap(parser.parse(text, fallbackSessionId: "f", defaultDate: epoch).first)
        XCTAssertEqual(s.firstUserPrompt, "Add a login screen please")
    }

    // MARK: - Tolerance

    func testMalformedLinesAreSkipped() {
        let text = """
        not json at all
        {"type":"user","sessionId":"s","message":{"content":"hello"}}
        {broken json
        """
        let sessions = parser.parse(text, fallbackSessionId: "f", defaultDate: epoch)
        XCTAssertEqual(sessions.count, 1)
        XCTAssertEqual(sessions.first?.firstUserPrompt, "hello")
    }

    func testFallbackSessionIdUsedWhenMissing() {
        let text = #"{"type":"user","message":{"content":"hi"}}"#
        let sessions = parser.parse(text, fallbackSessionId: "the-fallback", defaultDate: epoch)
        XCTAssertEqual(sessions.first?.sessionId, "the-fallback")
    }

    func testEmptyTextProducesNoSessions() {
        XCTAssertTrue(parser.parse("", fallbackSessionId: "f", defaultDate: epoch).isEmpty)
        XCTAssertTrue(parser.parse("\n\n  \n", fallbackSessionId: "f", defaultDate: epoch).isEmpty)
    }

    // MARK: - Pure helpers

    func testModelFamily() {
        XCTAssertEqual(JSONLParser.modelFamily(for: "claude-opus-4-7"), "Opus")
        XCTAssertEqual(JSONLParser.modelFamily(for: "claude-sonnet-4-6-20250514"), "Sonnet")
        XCTAssertEqual(JSONLParser.modelFamily(for: "claude-haiku-4-5"), "Haiku")
        XCTAssertEqual(JSONLParser.modelFamily(for: "gpt-4"), "Unknown")
        XCTAssertEqual(JSONLParser.modelFamily(for: ""), "Unknown")
    }

    func testParseTimestampWithAndWithoutFractionalSeconds() {
        XCTAssertNotNil(JSONLParser.parseTimestamp("2026-05-01T10:00:00.000Z"))
        XCTAssertNotNil(JSONLParser.parseTimestamp("2026-05-01T10:00:00Z"))
        XCTAssertNil(JSONLParser.parseTimestamp("not a date"))
    }

    func testExtractTextFromStringAndBlocks() {
        XCTAssertEqual(JSONLParser.extractText(fromContent: "  hello   world \n"), "hello world")
        let blocks: [[String: Any]] = [["type": "text", "text": "block text"]]
        XCTAssertEqual(JSONLParser.extractText(fromContent: blocks), "block text")
        let toolResult: [[String: Any]] = [["type": "tool_result", "content": "x"]]
        XCTAssertNil(JSONLParser.extractText(fromContent: toolResult))
        XCTAssertNil(JSONLParser.extractText(fromContent: nil))
    }
}
