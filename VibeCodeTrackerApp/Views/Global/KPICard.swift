import SwiftUI

/// Reusable metric card: a big value with a title and optional caption.
struct KPICard: View {
    let title: String
    let value: String
    var caption: String? = nil
    var systemImage: String? = nil
    /// Optional help tooltip (e.g. why a value is "—").
    var tooltip: String? = nil

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 4) {
                if let systemImage {
                    Image(systemName: systemImage)
                        .foregroundStyle(.tint)
                        .font(.caption)
                }
                Text(title.uppercased())
                    .font(.caption2.weight(.semibold))
                    .foregroundStyle(.secondary)
            }
            Text(value)
                .font(.system(.title, design: .rounded).weight(.semibold))
                .lineLimit(1)
                .minimumScaleFactor(0.6)
            if let caption {
                Text(caption)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(14)
        .background(.background.secondary, in: RoundedRectangle(cornerRadius: 12))
        .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(.separator, lineWidth: 0.5))
        .help(tooltip ?? "")
    }
}

#Preview {
    LazyVGrid(columns: [GridItem(.adaptive(minimum: 150))], spacing: 12) {
        KPICard(title: "Projects", value: "12", caption: "active", systemImage: "folder")
        KPICard(title: "Cost", value: "—", caption: "this week", systemImage: "dollarsign.circle",
                tooltip: "Configure API key in Settings")
    }
    .padding()
    .frame(width: 400)
}
