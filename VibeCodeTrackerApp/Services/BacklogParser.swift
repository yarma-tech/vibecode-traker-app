import Foundation

/// A backlog item parsed from markdown, before it touches the database.
struct ParsedBacklogItem: Equatable, Sendable {
    var title: String
    var isDone: Bool
    var priority: String?
    var lineNumber: Int
    var sourceFile: String
}

/// Parses markdown checkbox items from TODO.md / BACKLOG.md.
final class BacklogParser {
    /// Filenames checked at a project root, in priority order.
    static let candidateFiles = ["TODO.md", "BACKLOG.md", "todo.md", "backlog.md"]

    // MARK: - Pure parsing

    static func parse(_ text: String, sourceFile: String) -> [ParsedBacklogItem] {
        var items: [ParsedBacklogItem] = []
        var lineNumber = 0
        text.enumerateLines { line, _ in
            lineNumber += 1
            guard let (isDone, title) = parseCheckbox(line) else { return }
            items.append(
                ParsedBacklogItem(
                    title: title,
                    isDone: isDone,
                    priority: extractPriority(title),
                    lineNumber: lineNumber,
                    sourceFile: sourceFile
                )
            )
        }
        return items
    }

    /// Parses one line as a markdown checkbox. Returns (isDone, title) or nil.
    /// Accepts optional indentation and an optional `-`/`*`/`+` list marker:
    /// `- [ ] task`, `* [x] done`, `[ ] no-marker`.
    static func parseCheckbox(_ line: String) -> (isDone: Bool, title: String)? {
        var rest = Substring(line).drop { $0 == " " || $0 == "\t" }
        if let first = rest.first, "-*+".contains(first) {
            rest = rest.dropFirst().drop { $0 == " " || $0 == "\t" }
        }
        guard rest.count >= 3, rest.first == "[" else { return nil }
        let marker = rest[rest.index(rest.startIndex, offsetBy: 1)]
        guard rest[rest.index(rest.startIndex, offsetBy: 2)] == "]" else { return nil }
        let isDone: Bool
        switch marker {
        case " ": isDone = false
        case "x", "X": isDone = true
        default: return nil
        }
        let title = rest[rest.index(rest.startIndex, offsetBy: 3)...]
            .trimmingCharacters(in: .whitespaces)
        guard !title.isEmpty else { return nil }
        return (isDone, title)
    }

    /// Extracts a priority token (P0/P1/P2) anywhere in the title, including
    /// `**P0**` and `[P1]` forms.
    static func extractPriority(_ title: String) -> String? {
        guard let regex = priorityRegex else { return nil }
        let range = NSRange(title.startIndex..<title.endIndex, in: title)
        guard let match = regex.firstMatch(in: title, range: range),
              let r = Range(match.range(at: 1), in: title) else { return nil }
        return String(title[r]).uppercased()
    }

    // MARK: - Filesystem

    /// Reads candidate backlog files at a project root and parses them.
    func parseProject(at path: String) -> [ParsedBacklogItem] {
        let fileManager = FileManager.default
        var items: [ParsedBacklogItem] = []
        var usedNames = Set<String>()
        for name in Self.candidateFiles {
            // Avoid double-reading TODO.md and todo.md on case-insensitive FS.
            let lower = name.lowercased()
            guard !usedNames.contains(lower) else { continue }
            let fileURL = URL(fileURLWithPath: path).appendingPathComponent(name)
            guard fileManager.fileExists(atPath: fileURL.path),
                  let text = try? String(contentsOf: fileURL, encoding: .utf8) else { continue }
            usedNames.insert(lower)
            items.append(contentsOf: Self.parse(text, sourceFile: name))
        }
        return items
    }

    // MARK: - Internals

    private static let priorityRegex = try? NSRegularExpression(pattern: "\\b(P[0-2])\\b")
}
