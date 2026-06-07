import Foundation
import Security

/// Abstraction over secret storage so the UI and tests don't depend on the real
/// Keychain (see `InMemorySecretStore`).
protocol SecretStoring: AnyObject {
    func set(_ value: String) throws
    func get() throws -> String?
    func delete() throws
}

/// Stores a single secret (the Anthropic Admin API key) in the macOS Keychain.
final class KeychainStore: SecretStoring {
    enum KeychainError: Error, Equatable {
        case unexpectedStatus(OSStatus)
    }

    private let service: String
    private let account: String

    init(service: String = "tech.yannick.vibecodetracker",
         account: String = "anthropic-admin-api-key") {
        self.service = service
        self.account = account
    }

    private var baseQuery: [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account
        ]
    }

    func set(_ value: String) throws {
        let data = Data(value.utf8)
        let updateStatus = SecItemUpdate(baseQuery as CFDictionary,
                                         [kSecValueData as String: data] as CFDictionary)
        switch updateStatus {
        case errSecSuccess:
            return
        case errSecItemNotFound:
            var addQuery = baseQuery
            addQuery[kSecValueData as String] = data
            let addStatus = SecItemAdd(addQuery as CFDictionary, nil)
            guard addStatus == errSecSuccess else { throw KeychainError.unexpectedStatus(addStatus) }
        default:
            throw KeychainError.unexpectedStatus(updateStatus)
        }
    }

    func get() throws -> String? {
        var query = baseQuery
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne

        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        switch status {
        case errSecSuccess:
            guard let data = item as? Data else { return nil }
            return String(data: data, encoding: .utf8)
        case errSecItemNotFound:
            return nil
        default:
            throw KeychainError.unexpectedStatus(status)
        }
    }

    func delete() throws {
        let status = SecItemDelete(baseQuery as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw KeychainError.unexpectedStatus(status)
        }
    }
}

/// In-memory secret store for tests and SwiftUI previews.
final class InMemorySecretStore: SecretStoring {
    private var value: String?
    init(value: String? = nil) { self.value = value }
    func set(_ value: String) throws { self.value = value }
    func get() throws -> String? { value }
    func delete() throws { value = nil }
}
