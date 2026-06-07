import Foundation
import SwiftData

/// Creates the app's `ModelContainer`.
enum PersistenceController {
    /// Builds the on-disk (or in-memory) container.
    ///
    /// During active development the schema changes between slices. If the
    /// existing on-disk store is incompatible, it is wiped and recreated once
    /// rather than crashing the app. There is no production data to protect in V1.
    static func makeContainer(inMemory: Bool = false, cloudKit: Bool = false) -> ModelContainer {
        let schema = Schema(AppSchema.models)
        let configuration: ModelConfiguration
        if cloudKit {
            // Inert in V1 (CloudKitSupport.isAvailable == false). Kept here to show
            // the integration point; needs entitlements + removal of @Attribute(.unique).
            configuration = ModelConfiguration(schema: schema, cloudKitDatabase: .private(CloudKitSupport.containerID))
        } else {
            configuration = ModelConfiguration(schema: schema, isStoredInMemoryOnly: inMemory)
        }
        do {
            return try ModelContainer(for: schema, configurations: [configuration])
        } catch {
            Log.persistence.error("ModelContainer creation failed: \(error.localizedDescription, privacy: .public). Resetting store.")
            destroyStore(at: configuration.url)
            do {
                return try ModelContainer(for: schema, configurations: [configuration])
            } catch {
                // Unrecoverable: the app cannot run without persistence.
                fatalError("Unrecoverable ModelContainer failure: \(error)")
            }
        }
    }

    private static func destroyStore(at url: URL) {
        let fileManager = FileManager.default
        for suffix in ["", "-wal", "-shm"] {
            let target = URL(fileURLWithPath: url.path + suffix)
            try? fileManager.removeItem(at: target)
        }
    }
}
