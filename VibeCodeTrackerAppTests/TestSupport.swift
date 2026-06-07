import Foundation
import SwiftData
@testable import VibeCodeTrackerApp

/// Shared helpers for tests.
enum TestSupport {
    /// Creates an in-memory `ModelContainer` covering the full app schema.
    ///
    /// IMPORTANT: the caller MUST retain the returned container for the duration
    /// of the test. `container.mainContext` does **not** keep the container
    /// alive — using a context whose container has deallocated traps inside
    /// SwiftData (EXC_BREAKPOINT). Always write:
    /// `let container = try TestSupport.makeInMemoryContainer(); let ctx = container.mainContext`
    @MainActor
    static func makeInMemoryContainer() throws -> ModelContainer {
        try ModelContainer(
            for: Schema(AppSchema.models),
            configurations: ModelConfiguration(isStoredInMemoryOnly: true)
        )
    }

    /// Creates a throwaway directory mimicking `~/.claude/projects/` with the
    /// given sub-folders (project hashes) and loose files. Caller removes it.
    static func makeTempProjectsDir(folders: [String], files: [String] = []) throws -> URL {
        let fm = FileManager.default
        let root = fm.temporaryDirectory.appending(path: "vct-projects-\(UUID().uuidString)")
        try fm.createDirectory(at: root, withIntermediateDirectories: true)
        for folder in folders {
            try fm.createDirectory(at: root.appending(path: folder), withIntermediateDirectories: true)
        }
        for file in files {
            try Data().write(to: root.appending(path: file))
        }
        return root
    }

    /// URL of a committed fixture, resolved relative to this source file so it
    /// works from both CLI and CI without bundle-resource configuration.
    static func fixtureURL(_ name: String) -> URL {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .appendingPathComponent("Fixtures/\(name)")
    }

    static func fixtureText(_ name: String) throws -> String {
        try String(contentsOf: fixtureURL(name), encoding: .utf8)
    }
}
