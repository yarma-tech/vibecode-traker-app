import SwiftUI

/// App settings: cost-estimate note, iCloud sync toggle, refresh frequency.
struct SettingsView: View {
    @AppStorage(PreferenceKey.syncViaICloud) private var syncViaICloud = false
    @AppStorage(PreferenceKey.refreshFrequency) private var refreshFrequencyRaw = RefreshFrequency.hourly.rawValue

    private var appVersion: String {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "—"
    }

    var body: some View {
        Form {
            Section("Costs") {
                Text("Costs are estimated locally from each session's token counts at published Anthropic list prices. They always display — no account or API key is required, and nothing leaves your Mac.")
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
    }
}

#Preview {
    SettingsView()
        .frame(width: 480, height: 420)
}
