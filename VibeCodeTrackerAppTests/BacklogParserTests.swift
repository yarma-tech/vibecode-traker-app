import XCTest
@testable import VibeCodeTrackerApp

final class BacklogParserTests: XCTestCase {

    func testParsesCheckboxVariants() {
        let text = """
        # My TODO
        - [ ] Open task
        - [x] Done task
        * [ ] Star marker
        [ ] No marker
        - [X] Capital done
        regular text, not an item
        - not a checkbox
        """
        let items = BacklogParser.parse(text, sourceFile: "TODO.md")
        XCTAssertEqual(items.count, 5)
        XCTAssertEqual(items[0].title, "Open task")
        XCTAssertFalse(items[0].isDone)
        XCTAssertTrue(items[1].isDone)
        XCTAssertEqual(items[2].title, "Star marker")
        XCTAssertEqual(items[3].title, "No marker")
        XCTAssertTrue(items[4].isDone)
    }

    func testLineNumbersAreOneBased() {
        let text = "intro\n- [ ] first\n\n- [ ] second"
        let items = BacklogParser.parse(text, sourceFile: "TODO.md")
        XCTAssertEqual(items[0].lineNumber, 2)
        XCTAssertEqual(items[1].lineNumber, 4)
    }

    func testExtractPriority() {
        XCTAssertEqual(BacklogParser.extractPriority("**P0** urgent thing"), "P0")
        XCTAssertEqual(BacklogParser.extractPriority("[P1] another"), "P1")
        XCTAssertEqual(BacklogParser.extractPriority("do P2 later"), "P2")
        XCTAssertNil(BacklogParser.extractPriority("no priority here"))
        XCTAssertNil(BacklogParser.extractPriority("P3 is out of range"))
    }

    func testParseProjectReadsTodoFile() throws {
        let dir = FileManager.default.temporaryDirectory.appending(path: "vct-backlog-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }

        let content = "- [ ] **P0** Do thing\n- [x] Done thing\nnot an item\n"
        try content.write(to: dir.appendingPathComponent("TODO.md"), atomically: true, encoding: .utf8)

        let items = BacklogParser().parseProject(at: dir.path)
        XCTAssertEqual(items.count, 2)
        XCTAssertEqual(items[0].priority, "P0")
        XCTAssertEqual(items[0].sourceFile, "TODO.md")
        XCTAssertTrue(items[1].isDone)
    }
}
