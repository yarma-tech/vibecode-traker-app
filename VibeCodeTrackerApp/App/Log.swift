import Foundation
import os

/// Centralized `os.Logger` instances. Production code logs through these instead
/// of `print(...)`.
enum Log {
    private static let subsystem = Bundle.main.bundleIdentifier ?? "tech.yannick.vibecodetracker"

    static let app = Logger(subsystem: subsystem, category: "app")
    static let persistence = Logger(subsystem: subsystem, category: "persistence")
    static let scanner = Logger(subsystem: subsystem, category: "scanner")
    static let parser = Logger(subsystem: subsystem, category: "parser")
    static let git = Logger(subsystem: subsystem, category: "git")
    static let backlog = Logger(subsystem: subsystem, category: "backlog")
    static let stack = Logger(subsystem: subsystem, category: "stack")
    static let network = Logger(subsystem: subsystem, category: "network")
    static let keychain = Logger(subsystem: subsystem, category: "keychain")
}

/// Lightweight environment probes.
enum AppEnvironment {
    /// True when running inside an XCTest host. Used to skip background scanning
    /// of the real filesystem during tests.
    static var isRunningTests: Bool {
        ProcessInfo.processInfo.environment["XCTestConfigurationFilePath"] != nil
            || NSClassFromString("XCTestCase") != nil
    }
}
