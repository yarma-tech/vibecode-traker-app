import AppKit

/// Prevents two copies of the app (e.g. a Debug build run from Xcode and the
/// installed Release) from running at once.
///
/// Both copies share the same bundle identifier and therefore the same on-disk
/// SwiftData store. Two processes mutating one store orphan each other's
/// in-memory `@Model` instances — reading a stored property of an orphaned model
/// then traps the process (EXC_BREAKPOINT, "backing data could no longer be
/// found"). macOS apps are single-instance by convention; this enforces it even
/// across distinct bundle paths that share an identifier.
enum SingleInstanceGuard {
    /// If another running process has this app's bundle identifier, brings it to
    /// the front and returns `true` — the caller should exit *before* opening the
    /// store. Always `false` under tests (the XCTest host shares the identifier).
    static func anotherInstanceIsRunning() -> Bool {
        guard !AppEnvironment.isRunningTests else { return false }
        let me = NSRunningApplication.current
        guard let bundleID = me.bundleIdentifier else { return false }
        let others = NSWorkspace.shared.runningApplications.filter {
            $0.bundleIdentifier == bundleID && $0.processIdentifier != me.processIdentifier
        }
        guard let existing = others.first else { return false }
        existing.activate()
        return true
    }
}
