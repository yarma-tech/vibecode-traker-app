import SwiftUI

/// Placeholder root view shown during bootstrap (Slice 1).
///
/// This is replaced by the sidebar + dashboard navigation in Slice 2 onward.
struct ContentView: View {
    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "chart.bar.doc.horizontal")
                .font(.system(size: 48, weight: .light))
                .foregroundStyle(.tint)
            Text("Vibe Code Tracker — v0.1")
                .font(.title2.weight(.semibold))
            Text("Local dashboard for your Claude Code projects")
                .font(.callout)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}

#Preview {
    ContentView()
}
