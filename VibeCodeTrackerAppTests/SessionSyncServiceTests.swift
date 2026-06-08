import XCTest
import SwiftData
@testable import VibeCodeTrackerApp

final class SessionSyncServiceTests: XCTestCase {
    private let projectHash = "-Users-demo-Developer-login-app"
    private let sessionId = "11111111-1111-1111-1111-111111111111"

    @MainActor
    func testSyncCreatesSessionsAndRecoversRealPath() async throws {
        let temp = try makeProjectsDirWithFixture()
        defer { try? FileManager.default.removeItem(at: temp) }

        let container = try TestSupport.makeInMemoryContainer()
        defer { withExtendedLifetime(container) {} }
        let context = container.mainContext
        let service = SessionSyncService(projectsDirectory: temp)

        let summary = try await service.sync(into: context)
        XCTAssertEqual(summary.sessionsCreated, 1)

        let projects = try context.fetch(FetchDescriptor<Project>())
        XCTAssertEqual(projects.count, 1)
        let project = try XCTUnwrap(projects.first)
        // Real path recovered from the JSONL cwd, not the lossy folder decode.
        XCTAssertEqual(project.path, "/Users/demo/Developer/login-app")
        XCTAssertFalse(project.pathIsProvisional)
        XCTAssertEqual(project.name, "login-app")

        let sessions = try context.fetch(FetchDescriptor<Session>())
        XCTAssertEqual(sessions.count, 1)
        let session = try XCTUnwrap(sessions.first)
        XCTAssertEqual(session.outputTokens, 300)
        XCTAssertEqual(session.totalTokens, 3365)
        XCTAssertEqual(session.modelFamily, "Sonnet")
        XCTAssertEqual(session.project?.claudeProjectHash, projectHash)
    }

    @MainActor
    func testSyncIsIdempotentAndSkipsUnchangedFiles() async throws {
        let temp = try makeProjectsDirWithFixture()
        defer { try? FileManager.default.removeItem(at: temp) }

        let container = try TestSupport.makeInMemoryContainer()
        defer { withExtendedLifetime(container) {} }
        let context = container.mainContext
        let service = SessionSyncService(projectsDirectory: temp)

        let first = try await service.sync(into: context)
        XCTAssertEqual(first.sessionsCreated, 1)
        XCTAssertEqual(first.filesParsed, 1)

        // Second run: file unchanged → skipped, nothing parsed or created.
        let second = try await service.sync(into: context)
        XCTAssertEqual(second.sessionsCreated, 0)
        XCTAssertEqual(second.filesParsed, 0, "Unchanged file must be skipped on rescan")

        XCTAssertEqual(try context.fetch(FetchDescriptor<Session>()).count, 1)
        XCTAssertEqual(try context.fetch(FetchDescriptor<Project>()).count, 1)
    }

    // MARK: - Helpers

    private func makeProjectsDirWithFixture() throws -> URL {
        let root = try TestSupport.makeTempProjectsDir(folders: [projectHash])
        let fixture = try TestSupport.fixtureText("session-basic.jsonl")
        let dest = root.appending(path: projectHash).appending(path: "\(sessionId).jsonl")
        try fixture.write(to: dest, atomically: true, encoding: .utf8)
        return root
    }
}
