import SwiftUI

/// Large, prominent KPI card for the dashboard's "hero" tier.
struct HeroKPI: View {
    let title: String
    let value: String
    var caption: String? = nil
    let systemImage: String

    var body: some View {
        Card {
            VStack(alignment: .leading, spacing: Spacing.sm) {
                Label(title.uppercased(), systemImage: systemImage)
                    .font(.caption2.weight(.semibold)).foregroundStyle(.secondary)
                    .labelStyle(.titleAndIcon)
                Text(value).font(.system(.largeTitle, design: .rounded).weight(.semibold))
                    .lineLimit(1).minimumScaleFactor(0.6)
                if let caption { Text(caption).font(.caption2).foregroundStyle(.secondary) }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}

/// Compact label/value cell for the dashboard's secondary metric strip.
struct SecondaryMetric: View {
    let label: String
    let value: String

    var body: some View {
        VStack(spacing: 2) {
            Text(value).font(.headline.weight(.semibold)).monospacedDigit()
            Text(label.uppercased()).font(.caption2).foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity)
    }
}
