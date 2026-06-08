import XCTest
@testable import VibeCodeTrackerApp

final class SessionTitleGeneratorTests: XCTestCase {

    // MARK: - Empty / nil

    func testNilPromptFallsBack() {
        XCTAssertEqual(SessionTitleGenerator.heuristicTitle(from: nil),
                       SessionTitleGenerator.fallback)
    }

    func testEmptyAndWhitespacePromptFallBack() {
        XCTAssertEqual(SessionTitleGenerator.heuristicTitle(from: ""),
                       SessionTitleGenerator.fallback)
        XCTAssertEqual(SessionTitleGenerator.heuristicTitle(from: "   \n\t "),
                       SessionTitleGenerator.fallback)
    }

    func testPreambleOnlyFallsBack() {
        // "Please" alone strips to nothing.
        XCTAssertEqual(SessionTitleGenerator.heuristicTitle(from: "Please"),
                       SessionTitleGenerator.fallback)
    }

    // MARK: - Preamble stripping

    func testStripsPolitePreamble() {
        let title = SessionTitleGenerator.heuristicTitle(from: "Please can you fix the login bug")
        XCTAssertEqual(title, "Fix the login bug")
        XCTAssertFalse(title.lowercased().hasPrefix("please"))
        XCTAssertFalse(title.lowercased().contains("can you"))
    }

    func testPreambleRespectsWordBoundary() {
        // "so" must not be stripped from "sort".
        let title = SessionTitleGenerator.heuristicTitle(from: "sort the list")
        XCTAssertEqual(title, "Sort the list")
    }

    // MARK: - Caps

    func testCapsToSixWords() {
        let title = SessionTitleGenerator.heuristicTitle(
            from: "one two three four five six seven eight nine ten")
        XCTAssertLessThanOrEqual(title.split(separator: " ").count, SessionTitleGenerator.maxWords)
        XCTAssertEqual(title, "One two three four five six")
    }

    func testCapsToSixtyChars() {
        let longWord = String(repeating: "a", count: 100)
        let title = SessionTitleGenerator.heuristicTitle(from: longWord)
        XCTAssertLessThanOrEqual(title.count, SessionTitleGenerator.maxChars)
    }

    // MARK: - First clause

    func testTakesFirstClause() {
        let title = SessionTitleGenerator.heuristicTitle(from: "Add a button. Then deploy.")
        XCTAssertEqual(title, "Add a button")
    }

    // MARK: - sanitize (also used for LLM output)

    func testSanitizeStripsQuotesAndTrailingPunctuation() {
        XCTAssertEqual(SessionTitleGenerator.sanitize("\"Fix bug.\""), "Fix bug")
        XCTAssertEqual(SessionTitleGenerator.sanitize("`code`"), "Code")
        XCTAssertEqual(SessionTitleGenerator.sanitize("Refactor parser…"), "Refactor parser")
    }

    func testSanitizeCapitalizesFirstLetter() {
        XCTAssertEqual(SessionTitleGenerator.sanitize("migrate to swiftdata"), "Migrate to swiftdata")
    }

    func testSanitizeEmptyStaysEmpty() {
        XCTAssertEqual(SessionTitleGenerator.sanitize("   "), "")
        XCTAssertEqual(SessionTitleGenerator.sanitize("\"\""), "")
    }
}
