import SwiftUI
import SwiftData

/// Maps the current sidebar selection to a detail view. Destinations are filled
/// in by later slices (dashboard → Slice 4, project detail → Slice 5, settings →
/// Slice 10).
struct DetailRouter: View {
    let selection: SidebarItem?

    var body: some View {
        switch selection {
        case .dashboard, .none:
            GlobalDashboardView()
        case .project(let id):
            ProjectDetailView(projectID: id)
        case .settings:
            PlaceholderDetail(
                title: "Settings",
                systemImage: "gearshape",
                subtitle: "API key and preferences arrive in Slice 10."
            )
        }
    }
}

/// Generic centered placeholder for not-yet-built destinations.
struct PlaceholderDetail: View {
    let title: String
    let systemImage: String
    let subtitle: String?

    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: systemImage)
                .font(.system(size: 44, weight: .light))
                .foregroundStyle(.tint)
            Text(title).font(.title2.weight(.semibold))
            if let subtitle {
                Text(subtitle)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}
