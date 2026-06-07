import SwiftUI

/// A section title with an optional count badge and optional collapse chevron.
/// When `isExpanded` is bound, the whole header is a button that toggles it.
struct SectionHeader: View {
    let title: String
    var count: Int? = nil
    var isExpanded: Binding<Bool>? = nil

    var body: some View {
        if let isExpanded {
            Button {
                withAnimation(.snappy) { isExpanded.wrappedValue.toggle() }
            } label: { label(expanded: isExpanded.wrappedValue) }
            .buttonStyle(.plain)
        } else {
            label(expanded: true)
        }
    }

    private func label(expanded: Bool) -> some View {
        HStack(spacing: Spacing.sm) {
            if isExpanded != nil {
                Image(systemName: "chevron.right")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.secondary)
                    .rotationEffect(.degrees(expanded ? 90 : 0))
            }
            Text(title).font(.headline)
            if let count {
                Text("\(count)")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 1)
                    .background(.background.secondary, in: Capsule())
            }
            Spacer(minLength: 0)
        }
        .contentShape(Rectangle())
    }
}

#Preview {
    VStack(alignment: .leading, spacing: Spacing.lg) {
        SectionHeader(title: "Recent commits", count: 10)
        SectionHeader(title: "Backlog", count: 3, isExpanded: .constant(true))
        SectionHeader(title: "Recent sessions", count: 0, isExpanded: .constant(false))
    }
    .padding()
    .frame(width: 360)
}
