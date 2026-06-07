import Foundation
import SwiftData

struct DetectedStack: Sendable {
    let claudeProjectHash: String
    let tags: [String]
}

/// Detects each project's stack from disk and stores it on `Project.stack`.
final class StackSyncService {
    private let detector = StackDetector()

    func detectAll(candidates: [(hash: String, path: String)]) -> [DetectedStack] {
        candidates.map { DetectedStack(claudeProjectHash: $0.hash, tags: detector.detectStack(at: $0.path)) }
    }

    @MainActor
    @discardableResult
    func apply(_ detected: [DetectedStack], into context: ModelContext) throws -> Int {
        var projectsByHash: [String: Project] = [:]
        for project in try context.fetch(FetchDescriptor<Project>()) {
            projectsByHash[project.claudeProjectHash] = project
        }
        var changed = 0
        for item in detected {
            guard let project = projectsByHash[item.claudeProjectHash] else { continue }
            if project.stack != item.tags {
                project.stack = item.tags
                changed += 1
            }
        }
        if context.hasChanges { try context.save() }
        return changed
    }

    @MainActor
    func sync(into context: ModelContext) async throws {
        let candidates: [(hash: String, path: String)] = try context.fetch(FetchDescriptor<Project>())
            .filter { !$0.pathIsProvisional }
            .map { (hash: $0.claudeProjectHash, path: $0.path) }

        let detected = await Task.detached(priority: .utility) { [self] in
            detectAll(candidates: candidates)
        }.value

        let changed = try apply(detected, into: context)
        Log.stack.info("Stack detection updated \(changed) projects.")
    }
}
