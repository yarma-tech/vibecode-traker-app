import SwiftUI

/// App settings: Anthropic API key (Keychain), sync toggle, refresh frequency.
struct SettingsView: View {
    @AppStorage(PreferenceKey.syncViaICloud) private var syncViaICloud = false
    @AppStorage(PreferenceKey.refreshFrequency) private var refreshFrequencyRaw = RefreshFrequency.hourly.rawValue

    @State private var apiKey = ""
    @State private var status: StatusMessage?
    @State private var isTesting = false

    private let secretStore: SecretStoring = KeychainStore()

    private var appVersion: String {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "—"
    }

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

            Section {
                VStack(spacing: Spacing.sm) {
                    Image("LogoLockup")
                        .resizable()
                        .scaledToFit()
                        .frame(width: 168)
                        .accessibilityLabel("Vibe Code Tracker")
                    Text("Version \(appVersion)")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, Spacing.sm)
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

    private func testConnection() {
        let key = apiKey
        isTesting = true
        status = nil
        Task {
            let result = await AnthropicUsageClient().testConnection(apiKey: key)
            await MainActor.run {
                isTesting = false
                switch result {
                case .success:
                    status = StatusMessage(text: "Connection successful.", systemImage: "checkmark.circle", color: .green)
                case .failure(let error):
                    status = StatusMessage(text: Self.message(for: error), systemImage: "xmark.circle", color: .red)
                }
            }
        }
    }

    private static func message(for error: AnthropicAPIError) -> String {
        switch error {
        case .missingKey: return "Enter an API key first."
        case .unauthorized: return "Unauthorized — check that this is a valid Admin API key."
        case .rateLimited: return "Rate limited (429). Try again shortly."
        case .http(let code): return "Unexpected server response (HTTP \(code))."
        case .network(let message): return "Network error: \(message)"
        case .decoding: return "Could not read the API response."
        }
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
