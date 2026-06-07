import SwiftUI

/// App settings: Anthropic API key (Keychain), sync toggle, refresh frequency.
struct SettingsView: View {
    @AppStorage(PreferenceKey.syncViaICloud) private var syncViaICloud = false
    @AppStorage(PreferenceKey.refreshFrequency) private var refreshFrequencyRaw = RefreshFrequency.hourly.rawValue

    @State private var apiKey = ""
    @State private var status: StatusMessage?
    @State private var isTesting = false

    private let secretStore: SecretStoring = KeychainStore()

    var body: some View {
        Form {
            Section("Anthropic API") {
                SecureField("Admin API Key", text: $apiKey)
                    .textContentType(.password)
                HStack {
                    Button("Save", action: save)
                        .disabled(apiKey.isEmpty)
                    Button("Test connection", action: testConnection)
                        .disabled(apiKey.isEmpty || isTesting)
                    if isTesting { ProgressView().controlSize(.small) }
                    Spacer()
                    Button("Remove", role: .destructive, action: remove)
                }
                if let status {
                    Label(status.text, systemImage: status.systemImage)
                        .font(.caption)
                        .foregroundStyle(status.color)
                }
                Text("Your Admin API key is stored only in the macOS Keychain and used solely for api.anthropic.com. Costs stay hidden without it.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Section("Sync") {
                Toggle("Sync via iCloud", isOn: $syncViaICloud)
                Text("Syncs your dashboard across Macs via CloudKit. Requires an Apple Developer setup (see docs); enabling is wired in a later build.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Section("Refresh") {
                Picker("Frequency", selection: $refreshFrequencyRaw) {
                    ForEach(RefreshFrequency.allCases) { freq in
                        Text(freq.label).tag(freq.rawValue)
                    }
                }
            }
        }
        .formStyle(.grouped)
        .navigationTitle("Settings")
        .onAppear { apiKey = (try? secretStore.get()) ?? "" }
    }

    // MARK: - Actions

    private func save() {
        do {
            try secretStore.set(apiKey)
            status = StatusMessage(text: "API key saved to Keychain.", systemImage: "checkmark.circle", color: .green)
        } catch {
            status = StatusMessage(text: "Could not save key: \(error.localizedDescription)", systemImage: "xmark.circle", color: .red)
        }
    }

    private func remove() {
        do {
            try secretStore.delete()
            apiKey = ""
            status = StatusMessage(text: "API key removed.", systemImage: "trash", color: .secondary)
        } catch {
            status = StatusMessage(text: "Could not remove key: \(error.localizedDescription)", systemImage: "xmark.circle", color: .red)
        }
    }

    /// Slice 10: local validation only. Real network check is added in Slice 11.
    private func testConnection() {
        guard apiKey.hasPrefix("sk-ant-") else {
            status = StatusMessage(text: "That doesn't look like an Anthropic Admin key (expected to start with \"sk-ant-\").", systemImage: "exclamationmark.triangle", color: .orange)
            return
        }
        status = StatusMessage(text: "Key format looks valid. A live connection test is added with the usage API.", systemImage: "checkmark.circle", color: .green)
    }
}

private struct StatusMessage {
    let text: String
    let systemImage: String
    let color: Color
}

#Preview {
    SettingsView()
        .frame(width: 480, height: 420)
}
