# Architectural Decisions

Decisions taken autonomously during the overnight build. Each notes context,
the choice, alternatives, and consequences for Yannick to review.

## 2026-06-06 — Use XcodeGen to generate the project, commit the .xcodeproj

- **Context**: A buildable `.xcodeproj` is needed without a GUI Xcode session.
  The PRD (§5) prefers no XcodeGen "for OSS contribution simplicity", but the
  build prompt explicitly allows it.
- **Decision**: Use `xcodegen` (already installed at `/opt/homebrew/bin/xcodegen`)
  to generate the project from `project.yml`, **and commit the generated
  `.xcodeproj`**. Contributors can build with plain Xcode; only maintainers who
  change targets need xcodegen.
- **Alternatives**: hand-write `pbxproj` (rejected: fragile); SPM-only executable
  (rejected: limits app/SwiftData/asset capabilities).
- **Consequences**: `project.yml` is the source of truth for project structure;
  if you edit targets in Xcode, run `xcodegen generate` to keep them in sync.
  Documented in docs/DEVELOPMENT.md.

## 2026-06-06 — Swift 5 language mode (SWIFT_VERSION = 5.0)

- **Context**: Toolchain is Swift 6.3 (Xcode 26.5), which defaults to the Swift 6
  language mode with strict concurrency checking. SwiftData `@Model` + SwiftUI
  under strict concurrency generates significant friction (Sendable/actor errors).
- **Decision**: Pin `SWIFT_VERSION = 5.0`. The PRD specifies "Swift 5.9+", so this
  is in spec.
- **Consequences**: Smoother autonomous build. Migrating to the Swift 6 language
  mode is a future task once the data/UI layers are stable.

## 2026-06-06 — App is NOT sandboxed in V1

- **Context**: The app must read `~/.claude/projects/` (outside its container) and
  shell out to `/usr/bin/git`. App Sandbox blocks both without security-scoped
  bookmarks and would break `Process` execution.
- **Decision**: Ship V1 without App Sandbox / Hardened Runtime for the local build.
- **Consequences**: App Store distribution is out of scope for V1 anyway (PRD §4).
  A future sandboxed build would need security-scoped bookmarks for folder access
  and a different Git strategy. Documented as a known limitation.

## 2026-06-06 — Decode project paths from JSONL `cwd`, not the folder name

- **Context**: `~/.claude/projects/` folder names encode the cwd by replacing `/`
  with `-`. This is **lossy**: a real path like `/Users/.../agent-karata` and a
  hypothetical `/Users/.../agent/karata` both collapse to the same dashes. Verified
  against real data: folder `-Users-YarmaVideos-Developer-agent-karata` has JSONL
  `cwd = /Users/YarmaVideos/Developer/agent-karata`.
- **Decision**: Use the folder name only as `claudeProjectHash` (stable id). Derive
  the real `Project.path` from the JSONL `cwd` field (Slice 3). Slice 2 uses a naive
  `-`→`/` decode as a temporary display fallback until sessions are parsed.
- **Consequences**: Correct paths once any session is parsed; the naive decode is
  clearly marked as provisional.

## 2026-06-06 — Testing: XCTest, hosted by the app target

- **Context**: PRD mentions XCTest + Swift Testing. For reliability across CLI and
  CI, a single framework is simpler.
- **Decision**: Standardize on XCTest. The test target is hosted by the app target
  so `@testable import VibeCodeTrackerApp` works. App startup guards against running
  its background scan under XCTest (added in Slice 2) to keep tests hermetic.
- **Consequences**: Tests never touch Yannick's real filesystem — they use
  `FileManager.default.temporaryDirectory` and in-memory `ModelContainer`s.
