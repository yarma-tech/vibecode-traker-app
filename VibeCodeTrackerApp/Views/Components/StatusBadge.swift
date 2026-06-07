import SwiftUI

/// Colored dot + label for a session status.
struct StatusBadge: View {
    let status: String
    var showsLabel: Bool = true

    private var info: (color: Color, label: String) {
        switch status {
        case SessionStatus.inProgress.rawValue: return (.yellow, "In progress")
        case SessionStatus.blocked.rawValue: return (.red, "Blocked")
        default: return (.green, "Completed")
        }
    }

    var body: some View {
        let info = info
        HStack(spacing: 5) {
            Circle().fill(info.color).frame(width: 7, height: 7)
            if showsLabel {
                Text(info.label)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
        .accessibilityLabel(info.label)
    }
}
