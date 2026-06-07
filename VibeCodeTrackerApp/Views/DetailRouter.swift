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
            PlaceholderDetail(
                title: "Global Dashboard",
                systemImage: "square.grid.2x2",
                subtitle: "KPIs and activity charts arrive in Slice 4."
            )
        case .project(let id):
            ProjectStubView(projectID: id)
        case .settings:
            PlaceholderDetail(
                title: "Settings",
                systemImage: "gearshape",
                subtitle: "API key and preferences arrive in Slice 10."
            )
        }
    }
}

/// Interim single-project detail until the full view lands in Slice 5.
/// Shows project header + the parsed sessions list.
struct ProjectStubView: View {
    let projectID: PersistentIdentifier
    @Environment(\.modelContext) private var context

    var body: some View {
        if let project = context.model(for: projectID) as? Project {
            VStack(alignment: .leading, spacing: 8) {
                Text(project.name)
                    .font(.largeTitle.weight(.semibold))
                Text(project.path)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
                HStack(spacing: 12) {
                    Label("\(project.sessions.count) sessions", systemImage: "bubble.left.and.bubble.right")
                    Label("\(Format.tokens(project.sessions.reduce(0) { $0 + $1.totalTokens })) tok", systemImage: "number")
                }
                .font(.caption)
                .foregroundStyle(.secondary)

                Divider().padding(.vertical, 4)

                SessionsListView(sessions: project.sessions.sorted { $0.startedAt > $1.startedAt })
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .padding(24)
            .navigationTitle(project.name)
        } else {
            PlaceholderDetail(title: "Project not found", systemImage: "questionmark.folder", subtitle: nil)
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
