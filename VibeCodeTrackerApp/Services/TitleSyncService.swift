import Foundation
import SwiftData

/// Synthesizes a short `title` for sessions that don't have one yet.
///
/// Incremental: only processes `title == nil`, never regenerates (so titles
/// survive re-sync and a generated title is never clobbered). Bounded per run
/// because on-device generation is slow and serialized — a large backlog drains
/// over successive syncs. Mirrors the `StatusSyncService` shape.
@MainActor
final class TitleSyncService {
    /// Max sessions titled per sync pass (newest-first).
    nonisolated static let perRunLimit = 12

    @discardableResult
    func syncTitles(in context: ModelContext, limit: Int = perRunLimit) async throws -> Int {
        // 1. Fetch untitled sessions, newest first, capped.
        var descriptor = FetchDescriptor<Session>(
            predicate: #Predicate { $0.title == nil },
            sortBy: [SortDescriptor(\.startedAt, order: .reverse)]
        )
        descriptor.fetchLimit = limit
        let untitled = try context.fetch(descriptor)
        guard !untitled.isEmpty else { return 0 }

        // 2. Snapshot to value types — never hand @Model objects to the generator.
        let work = untitled.map { (id: $0.sessionId, prompt: $0.firstUserPrompt) }

        // 3. Generate off the main actor (LLM if available, else heuristic).
        let titles = await Self.generateTitles(for: work)

        // 4. Persist on the main actor, by id, re-checking title == nil (idempotent
        //    against a concurrent sync).
        let byId = Dictionary(untitled.map { ($0.sessionId, $0) }, uniquingKeysWith: { a, _ in a })
        var changed = 0
        for (id, title) in titles {
            guard let session = byId[id], session.title == nil, !title.isEmpty else { continue }
            session.title = title
            changed += 1
        }
        if context.hasChanges { try context.save() }
        Log.parser.info("Title sync generated \(changed) titles.")
        return changed
    }

    /// Generate titles for a batch of value-type work items. Uses the on-device
    /// model when available, otherwise the pure heuristic. `nonisolated` so the
    /// generation loop does not run on the main actor.
    nonisolated static func generateTitles(
        for work: [(id: String, prompt: String?)]
    ) async -> [String: String] {
        var out: [String: String] = [:]
        #if canImport(FoundationModels)
        if #available(macOS 26, *), FoundationModelTitler.isAvailable {
            let titler = FoundationModelTitler()
            for item in work {
                let prompt = item.prompt ?? ""
                if !prompt.isEmpty, let llm = await titler.title(forPrompt: prompt) {
                    out[item.id] = llm          // LLM succeeded
                } else {
                    out[item.id] = SessionTitleGenerator.heuristicTitle(from: item.prompt)
                }
            }
            return out
        }
        #endif
        // Fallback path: pure, instant, runs everywhere.
        for item in work {
            out[item.id] = SessionTitleGenerator.heuristicTitle(from: item.prompt)
        }
        return out
    }
}
