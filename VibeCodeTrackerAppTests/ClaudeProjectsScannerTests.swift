import XCTest
import SwiftData
@testable import VibeCodeTrackerApp

final class ClaudeProjectsScannerTests: XCTestCase {

    // MARK: - Pure decoding

    func testDecodePathReplacesDashesWithSlashes() {
        XCTAssertEqual(
            ClaudeProjectsScanner.decodePath(fromHash: "-Users-yann-projects-demo"),
            "/Users/yann/projects/demo"
        )
    }

    func testDecodePathHandlesEmptyHash() {
        XCTAssertEqual(ClaudeProjectsScanner.decodePath(fromHash: ""), "")
    }

    func testDisplayNameUsesLastComponent() {
        XCTAssertEqual(ClaudeProjectsScanner.displayName(forPath: "/Users/yann/projects/demo"), "demo")
        XCTAssertEqual(ClaudeProjectsScanner.displayName(forPath: "/Users/yann/projects/demo/"), "demo")
        XCTAssertEqual(ClaudeProjectsScanner.displayName(forPath: ""), "(unknown)")
    }

    // MARK: - Discovery against a temp fixture directory

    func testDiscoverProjectsFindsDirectoriesAndIgnoresFiles() throws {
        let temp = try TestSupport.makeTempProjectsDir(
            folders: ["-Users-test-alpha", "-Users-test-beta"],
            files: ["loose-file.txt"]
        )
        defer { try? FileManager.default.removeItem(at: temp) }

        let scanner = ClaudeProjectsScanner(projectsDirectory: temp)
        let discovered = try scanner.discoverProjects()

        let hashes = Set(discovered.map(\.claudeProjectHash))
        XCTAssertEqual(hashes, ["-Users-test-alpha", "-Users-test-beta"])
        XCTAssertEqual(discovered.count, 2)
    }

    func testDiscoverProjectsReturnsEmptyWhenDirectoryMissing() throws {
        let missing = FileManager.default.temporaryDirectory
            .appending(path: "vct-missing-\(UUID().uuidString)")
        let scanner = ClaudeProjectsScanner(projectsDirectory: missing)
        XCTAssertEqual(try scanner.discoverProjects(), [])
    }

    // MARK: - Upsert idempotency

    @MainActor
    func testScanIsIdempotent() async throws {
        let temp = try TestSupport.makeTempProjectsDir(folders: ["-Users-test-alpha", "-Users-test-beta"])
        defer { try? FileManager.default.removeItem(at: temp) }

        // Retain the container for the whole test (see TestSupport docs).
        let container = try TestSupport.makeInMemoryContainer()
        let context = container.mainContext
        let scanner = ClaudeProjectsScanner(projectsDirectory: temp)

        let first = try await scanner.scan(into: context)
        XCTAssertEqual(first.created, 2)
        XCTAssertEqual(first.updated, 0)

        let second = try await scanner.scan(into: context)
        XCTAssertEqual(second.created, 0)
        XCTAssertEqual(second.updated, 2)

        let all = try context.fetch(FetchDescriptor<Project>())
        XCTAssertEqual(all.count, 2, "Re-scanning must not create duplicate projects")
    }
}
