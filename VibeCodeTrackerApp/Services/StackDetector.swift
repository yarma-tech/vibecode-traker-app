import Foundation

/// Filesystem signals collected at a project root, used to infer the stack.
/// Keeping this as a value type makes `tags(from:)` pure and testable.
struct StackSignals: Equatable {
    var rootEntries: Set<String> = []
    var hasXcodeproj: Bool = false
    var packageJSON: Data? = nil
    var envContents: String? = nil
    var hasSupabaseConfig: Bool = false
    var hasPrismaSchema: Bool = false
}

/// Infers a project's tech stack from marker files and `package.json` deps,
/// following the PRD §8 mapping.
final class StackDetector {

    // MARK: - Pure inference

    static func tags(from signals: StackSignals) -> [String] {
        var tags: [String] = []
        func add(_ tag: String) { if !tags.contains(tag) { tags.append(tag) } }

        // Languages / ecosystems by marker file.
        if signals.rootEntries.contains("package.json") { add("JavaScript") }
        if signals.rootEntries.contains("tsconfig.json") { add("TypeScript") }
        if signals.rootEntries.contains("Cargo.toml") { add("Rust") }
        if signals.rootEntries.contains("pyproject.toml") || signals.rootEntries.contains("requirements.txt") { add("Python") }
        if signals.rootEntries.contains("go.mod") { add("Go") }
        if signals.rootEntries.contains("Podfile") || signals.hasXcodeproj { add("Swift") }

        // package.json dependencies → frameworks.
        if let data = signals.packageJSON {
            for tag in dependencyTags(fromPackageJSON: data) { add(tag) }
        }

        // Services / infra by marker file.
        if signals.hasSupabaseConfig { add("Supabase") }
        if signals.hasPrismaSchema { add("Prisma") }
        if signals.rootEntries.contains("Dockerfile") { add("Docker") }
        if let env = signals.envContents, env.contains("TWILIO_") { add("Twilio") }

        return tags
    }

    static func dependencyTags(fromPackageJSON data: Data) -> [String] {
        guard let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return [] }
        var deps: [String] = []
        for key in ["dependencies", "devDependencies", "peerDependencies"] {
            if let map = object[key] as? [String: Any] { deps.append(contentsOf: map.keys) }
        }

        var tags: [String] = []
        func add(_ tag: String) { if !tags.contains(tag) { tags.append(tag) } }
        for dep in deps {
            switch dep.lowercased() {
            case "next": add("Next.js")
            case "react": add("React")
            case "react-native": add("React Native")
            case "vue": add("Vue")
            case "svelte", "@sveltejs/kit": add("Svelte")
            case "@angular/core": add("Angular")
            case "express": add("Express")
            case "mastra": add("Mastra")
            case "tailwindcss": add("Tailwind")
            case "@supabase/supabase-js": add("Supabase")
            case "prisma", "@prisma/client": add("Prisma")
            case "twilio": add("Twilio")
            case "inngest": add("Inngest")
            case "remotion": add("Remotion")
            case "typescript": add("TypeScript")
            case "vite": add("Vite")
            default:
                let d = dep.lowercased()
                if d.hasPrefix("@mastra/") { add("Mastra") }
                else if d.hasPrefix("@remotion/") { add("Remotion") }
            }
        }
        return tags
    }

    // MARK: - Filesystem entry point

    func detectStack(at path: String) -> [String] {
        let fileManager = FileManager.default
        let root = URL(fileURLWithPath: path)
        let entries = (try? fileManager.contentsOfDirectory(atPath: path)) ?? []

        func exists(_ relative: String) -> Bool {
            fileManager.fileExists(atPath: root.appendingPathComponent(relative).path)
        }
        func data(_ relative: String) -> Data? {
            let url = root.appendingPathComponent(relative)
            return fileManager.fileExists(atPath: url.path) ? try? Data(contentsOf: url) : nil
        }

        let signals = StackSignals(
            rootEntries: Set(entries),
            hasXcodeproj: entries.contains { $0.hasSuffix(".xcodeproj") },
            packageJSON: data("package.json"),
            envContents: data(".env").flatMap { String(data: $0, encoding: .utf8) },
            hasSupabaseConfig: exists("supabase/config.toml"),
            hasPrismaSchema: exists("prisma/schema.prisma")
        )
        return Self.tags(from: signals)
    }
}
