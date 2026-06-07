import XCTest
@testable import VibeCodeTrackerApp

final class KeychainStoreTests: XCTestCase {

    // MARK: - In-memory store (hermetic)

    func testInMemoryRoundTrip() throws {
        let store = InMemorySecretStore()
        XCTAssertNil(try store.get())

        try store.set("sk-ant-test-123")
        XCTAssertEqual(try store.get(), "sk-ant-test-123")

        try store.set("sk-ant-updated")
        XCTAssertEqual(try store.get(), "sk-ant-updated")

        try store.delete()
        XCTAssertNil(try store.get())
    }

    // MARK: - Real Keychain (tolerant: skips if unavailable in this environment)

    func testKeychainRoundTrip() throws {
        let store = KeychainStore(service: "tech.yannick.vibecodetracker.tests",
                                  account: "test-\(UUID().uuidString)")
        do {
            try store.set("secret-value")
            XCTAssertEqual(try store.get(), "secret-value")
            try store.delete()
            XCTAssertNil(try store.get())
        } catch let KeychainStore.KeychainError.unexpectedStatus(status) {
            throw XCTSkip("Keychain not available in this environment (OSStatus \(status)).")
        }
    }
}
