import SwiftUI

/// Horizontal row of stack tag chips (e.g. "TypeScript", "Supabase").
/// Populated by the StackDetector in Slice 8.
struct StackTagsView: View {
    let stack: [String]

    var body: some View {
        if stack.isEmpty {
            Text("No stack detected yet")
                .font(.caption)
                .foregroundStyle(.secondary)
        } else {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 6) {
                    ForEach(stack, id: \.self) { tag in
                        TagChip(text: tag)
                    }
                }
            }
        }
    }
}

struct TagChip: View {
    let text: String

    var body: some View {
        Text(text)
            .font(.caption.weight(.medium))
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .background(.tint.opacity(0.15), in: Capsule())
            .foregroundStyle(.tint)
    }
}
