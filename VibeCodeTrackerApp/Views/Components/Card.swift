import SwiftUI

/// Reusable card chrome: secondary-background fill + hairline separator border
/// at a consistent corner radius. Single source of the app's card look.
struct Card<Content: View>: View {
    var padding: CGFloat = 14
    @ViewBuilder var content: () -> Content

    var body: some View {
        content()
            .padding(padding)
            .background(.background.secondary, in: RoundedRectangle(cornerRadius: 12))
            .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(.separator, lineWidth: 0.5))
    }
}

#Preview {
    Card {
        Text("Card content")
            .frame(maxWidth: .infinity, alignment: .leading)
    }
    .padding()
    .frame(width: 240)
}
