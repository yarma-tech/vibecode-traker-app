import Foundation

/// Which representation the project's "Recent sessions" section shows.
/// Stored as `rawValue` in `@AppStorage`.
enum SessionDisplayMode: String, CaseIterable, Identifiable {
    case table
    case kanban

    var id: String { rawValue }

    var label: String {
        switch self {
        case .table: return "Table"
        case .kanban: return "Kanban"
        }
    }

    var systemImage: String {
        switch self {
        case .table: return "tablecells"
        case .kanban: return "rectangle.split.3x1"
        }
    }
}
