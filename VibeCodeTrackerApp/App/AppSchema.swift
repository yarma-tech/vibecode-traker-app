import SwiftData

/// Single source of truth for the persisted model graph.
///
/// Extend `models` as new `@Model` types are introduced per slice. Keeping the
/// list in one place means the `ModelContainer` and every test container stay in
/// sync automatically.
enum AppSchema {
    static let models: [any PersistentModel.Type] = [
        Project.self
    ]
}
