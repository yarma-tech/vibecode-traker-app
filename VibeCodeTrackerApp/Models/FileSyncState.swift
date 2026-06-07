import Foundation
import SwiftData

/// Per-file bookkeeping for incremental scanning.
///
/// V1 works at file granularity: a `.jsonl` file is re-parsed only when its size
/// or modification date changes since the last scan. (Line-level incremental
/// aggregation — re-reading only appended bytes from `lastOffset` — is a future
/// refinement; `lastOffset` currently mirrors the file size.)
///
/// This state is intentionally local and never synced (it describes a specific
/// machine's filesystem).
@Model
final class FileSyncState {
    @Attribute(.unique) var path: String
    var lastOffset: Int
    var lastModified: Date

    init(path: String, lastOffset: Int, lastModified: Date) {
        self.path = path
        self.lastOffset = lastOffset
        self.lastModified = lastModified
    }
}
