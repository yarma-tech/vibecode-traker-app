import SwiftUI

/// Brief launch splash showing the full logo lockup, faded out shortly after
/// the window appears. Presented as an overlay by `ContentView`; never shown
/// under tests.
struct LaunchSplashView: View {
    var body: some View {
        ZStack {
            Color(nsColor: .windowBackgroundColor)
                .ignoresSafeArea()
            Image("LogoLockup")
                .resizable()
                .scaledToFit()
                .frame(width: 280)
                .accessibilityLabel("Vibe Code Tracker")
        }
    }
}

#Preview {
    LaunchSplashView()
        .frame(width: 600, height: 420)
}
