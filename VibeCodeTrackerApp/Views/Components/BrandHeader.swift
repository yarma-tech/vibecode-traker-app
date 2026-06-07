import SwiftUI

/// Compact brand mark shown at the top of the sidebar. Uses the monogram only
/// (the window title already carries the full "Vibe Code Tracker" name), with a
/// hairline divider separating it from the navigation list.
struct BrandHeader: View {
    var body: some View {
        VStack(spacing: 0) {
            Image("LogoMark")
                .resizable()
                .scaledToFit()
                .frame(height: 26)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 14)
                .padding(.top, Spacing.md)
                .padding(.bottom, Spacing.sm)
                .accessibilityLabel("Vibe Code Tracker")
            Divider()
        }
    }
}

#Preview {
    BrandHeader()
        .frame(width: 240)
}
