import Foundation
#if canImport(FoundationModels)
import FoundationModels

/// On-device LLM title generation via Apple's Foundation Models framework.
///
/// macOS 26+ only. The whole type is compiled out on SDKs without the framework
/// (`#if canImport`) and gated to macOS 26 at runtime (`@available`), so the
/// macOS 14.0 deployment target never resolves these symbols. The framework is
/// weak-linked (see project.yml) so the app still launches on macOS 14–15.
@available(macOS 26, *)
struct FoundationModelTitler {

    /// True only when the system model is usable right now: Apple Intelligence
    /// enabled, supported hardware, model downloaded and ready.
    static var isAvailable: Bool {
        if case .available = SystemLanguageModel.default.availability { return true }
        return false
    }

    /// Returns a short (≤6-word) title for the prompt, or `nil` on any failure
    /// (empty input, model error) so the caller can fall back to the heuristic.
    func title(forPrompt prompt: String) async -> String? {
        let trimmed = prompt.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return nil }

        let instructions = """
            You write extremely short titles for coding-assistant sessions. Given the \
            user's first message, reply with ONLY a title of at most six words naming \
            the main task. No quotes, no trailing punctuation, no preamble, no \
            explanation. Use the imperative mood, e.g. "Fix login redirect bug".
            """
        let session = LanguageModelSession(instructions: instructions)
        let options = GenerationOptions(temperature: 0.3, maximumResponseTokens: 24)
        do {
            let response = try await session.respond(to: trimmed, options: options)
            let title = SessionTitleGenerator.sanitize(response.content)
            return title.isEmpty ? nil : title
        } catch {
            Log.parser.notice("FoundationModels titling failed: \(error.localizedDescription, privacy: .public)")
            return nil
        }
    }
}
#endif
