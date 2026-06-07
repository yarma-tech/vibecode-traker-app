import XCTest
@testable import VibeCodeTrackerApp

final class StackDetectorTests: XCTestCase {

    func testMarkerFiles() {
        let signals = StackSignals(
            rootEntries: ["tsconfig.json", "Dockerfile", "go.mod"],
            hasXcodeproj: true
        )
        let tags = StackDetector.tags(from: signals)
        XCTAssertTrue(tags.contains("TypeScript"))
        XCTAssertTrue(tags.contains("Docker"))
        XCTAssertTrue(tags.contains("Go"))
        XCTAssertTrue(tags.contains("Swift")) // from hasXcodeproj
    }

    func testTwilioFromEnv() {
        let signals = StackSignals(rootEntries: [".env"], envContents: "TWILIO_AUTH_TOKEN=abc\nOTHER=1")
        XCTAssertTrue(StackDetector.tags(from: signals).contains("Twilio"))
    }

    func testDependencyTagsFromPackageJSON() throws {
        let json = """
        {
          "dependencies": { "next": "14", "react": "18", "@mastra/core": "1", "@supabase/supabase-js": "2", "twilio": "4" },
          "devDependencies": { "typescript": "5", "tailwindcss": "3" }
        }
        """
        let tags = StackDetector.dependencyTags(fromPackageJSON: Data(json.utf8))
        XCTAssertEqual(Set(tags), ["Next.js", "React", "Mastra", "Supabase", "Twilio", "TypeScript", "Tailwind"])
    }

    func testDeduplicates() {
        // typescript via tsconfig AND via package.json dep → single tag.
        let json = #"{"devDependencies":{"typescript":"5"}}"#
        let signals = StackSignals(rootEntries: ["package.json", "tsconfig.json"], packageJSON: Data(json.utf8))
        let tags = StackDetector.tags(from: signals)
        XCTAssertEqual(tags.filter { $0 == "TypeScript" }.count, 1)
        XCTAssertTrue(tags.contains("JavaScript"))
    }

    func testDetectStackOnTempDir() throws {
        let dir = FileManager.default.temporaryDirectory.appending(path: "vct-stack-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        try #"{"dependencies":{"react":"18"}}"#.write(to: dir.appendingPathComponent("package.json"), atomically: true, encoding: .utf8)
        try "".write(to: dir.appendingPathComponent("tsconfig.json"), atomically: true, encoding: .utf8)

        let tags = StackDetector().detectStack(at: dir.path)
        XCTAssertTrue(tags.contains("JavaScript"))
        XCTAssertTrue(tags.contains("TypeScript"))
        XCTAssertTrue(tags.contains("React"))
    }
}
