import Foundation
import SwiftData
@testable import VibeCodeTrackerApp

/// Shared helpers for tests.
enum TestSupport {
    /// Creates an in-memory `ModelContainer` covering the full app schema.
    ///
    /// IMPORTANT: the caller MUST keep the returned container alive for the whole
    /// test. `container.mainContext` does **not** retain its container, so a bare
    /// `let container = …; let ctx = container.mainContext` is NOT enough: ARC may
    /// release the container at its last syntactic use (the `.mainContext` line),
    /// especially in `async` tests where nothing references it again across an
    /// `await`. A later `ctx.fetch` then traps inside SwiftData (EXC_BREAKPOINT).
    /// Pin the lifetime to the end of scope right after creating it:
    /// ```
    /// let container = try TestSupport.makeInMemoryContainer()
    /// defer { withExtendedLifetime(container) {} }
    /// let context = container.mainContext
    /// ```
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
