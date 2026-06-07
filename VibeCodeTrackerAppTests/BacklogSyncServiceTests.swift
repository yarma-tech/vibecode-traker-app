import XCTest
import SwiftData
@testable import VibeCodeTrackerApp

final class BacklogSyncServiceTests: XCTestCase {
    private let projectHash = "-tmp-p"

    private func item(_ title: String, _ done: Bool, priority: String? = nil) -> ParsedBacklogItem {
        ParsedBacklogItem(title: title, isDone: done, priority: priority, lineNumber: 1, sourceFile: "TODO.md")
    }

    @MainActor
    func testReconcileCreateUpdateRemove() throws {
        let container = try TestSupport.makeInMemoryContainer()
        let context = container.mainContext
        let project = Project(name: "p", path: "/tmp/p", claudeProjectHash: projectHash, pathIsProvisional: false)
        context.insert(project)
        try context.save()

        let service = BacklogSyncService()

        // First pass: two items created.
        let s1 = try service.apply([ProjectBacklog(claudeProjectHash: projectHash, items: [item("A", false), item("B", true)])], into: context)
        XCTAssertEqual(s1.created, 2)
        XCTAssertEqual(try context.fetch(FetchDescriptor<BacklogItem>()).count, 2)

        // Second pass: B removed, A flipped to done.
        let s2 = try service.apply([ProjectBacklog(claudeProjectHash: projectHash, items: [item("A", true)])], into: context)
        XCTAssertEqual(s2.updated, 1)
        XCTAssertEqual(s2.removed, 1)

        let remaining = try context.fetch(FetchDescriptor<BacklogItem>())
        XCTAssertEqual(remaining.count, 1)
        XCTAssertEqual(remaining.first?.title, "A")
        XCTAssertEqual(remaining.first?.isDone, true)
    }
}
