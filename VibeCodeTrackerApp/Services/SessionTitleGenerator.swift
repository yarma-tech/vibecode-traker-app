import Foundation

/// Pure, synchronous title synthesis. No SwiftData, no network, no async.
///
/// Serves two roles:
/// 1. The fallback when the on-device model is unavailable (macOS 14–15, or
///    Apple Intelligence off / unsupported hardware).
/// 2. The shared `sanitize(_:)` cleanup applied to *both* heuristic and LLM
///    output, so titles look identical whatever produced them.
enum SessionTitleGenerator {
    static let maxWords = 6
    static let maxChars = 60
    static let fallback = "Untitled session"

    /// Leading filler stripped from a prompt before titling. Order matters:
    /// longer phrases are checked first so "can you" wins over a future "can".
    private static let preambles = [
        "i would like to", "i'd like to", "i want to", "i need to",
        "could you please", "can you please", "would you please",
        "could you", "can you", "would you", "will you",
        "please", "help me", "let's", "lets", "now", "hey",
        "hello", "hi", "okay", "ok", "so", "just"
    ]

    /// Derive a short title from the first user prompt.
    static func heuristicTitle(from prompt: String?) -> String {
        guard let raw = prompt?.trimmingCharacters(in: .whitespacesAndNewlines),
              !raw.isEmpty else { return fallback }

        let clause = firstClause(of: raw)
        let stripped = stripPreamble(clause)
        let title = sanitize(stripped)
        return title.isEmpty ? fallback : title
    }

    /// Shared cleanup for any title source: trims wrapping quotes/punctuation,
    /// collapses whitespace, caps to `maxWords`/`maxChars`, capitalizes.
    static func sanitize(_ text: String) -> String {
        let junk = CharacterSet(charactersIn: "\"'`.…,;:!?-—()[]{}")
        var s = text
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .trimmingCharacters(in: junk)
            .trimmingCharacters(in: .whitespacesAndNewlines)

        let words = s.split(whereSeparator: { $0 == " " || $0 == "\n" || $0 == "\t" })
        s = words.prefix(maxWords).joined(separator: " ")
        if s.count > maxChars {
            s = String(s.prefix(maxChars)).trimmingCharacters(in: .whitespaces)
        }
        guard let first = s.first else { return "" }
        return first.uppercased() + s.dropFirst()
    }

    /// First sentence/clause — text up to the first `. ! ? ; newline —`.
    /// (Colon is intentionally excluded so "Note: fix login" keeps its content.)
    static func firstClause(of text: String) -> String {
        let separators = CharacterSet(charactersIn: ".!?;\n—")
        if let range = text.rangeOfCharacter(from: separators) {
            return String(text[text.startIndex..<range.lowerBound])
        }
        return text
    }

    /// Strip leading polite/filler preambles, repeatedly, respecting word
    /// boundaries (so "sort the list" is never read as the preamble "so").
    static func stripPreamble(_ text: String) -> String {
        var s = text.trimmingCharacters(in: .whitespacesAndNewlines)
        var changed = true
        while changed {
            changed = false
            let lower = s.lowercased()
            for phrase in preambles where lower.hasPrefix(phrase) {
                let idx = s.index(s.startIndex, offsetBy: phrase.count)
                // Boundary: the prompt must not continue the word (e.g. "okayish").
                if idx == s.endIndex || !s[idx].isLetter {
                    s = String(s[idx...])
                        .trimmingCharacters(in: CharacterSet(charactersIn: " ,:;-—"))
                        .trimmingCharacters(in: .whitespacesAndNewlines)
                    changed = true
                    break
                }
            }
        }
        return s
    }
}
