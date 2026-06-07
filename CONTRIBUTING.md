# Contributing to Vibe Code Tracker

Thanks for your interest! This is an open-source macOS app built with Swift, SwiftUI, and SwiftData.

## Development setup

1. Install Xcode 15+ (developed with Xcode 26) and the command-line tools.
2. Clone the repo and open the project:
   ```bash
   git clone https://github.com/OWNER/vibecode-traker-app.git
   cd vibecode-traker-app
   open VibeCodeTrackerApp.xcodeproj
   ```
3. Press Run (⌘R), or use the command line:
   ```bash
   xcodebuild -scheme VibeCodeTrackerApp -destination 'platform=macOS' build
   xcodebuild -scheme VibeCodeTrackerApp -destination 'platform=macOS' test
   ```

The `.xcodeproj` is committed, so you don't need any extra tooling to build. If you change targets or files and use [XcodeGen](https://github.com/yonaskolb/XcodeGen), edit `project.yml` and run `xcodegen generate` (see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)).

## Coding conventions

- **Language**: all code comments in English (international project).
- **Commits**: [Conventional Commits](https://www.conventionalcommits.org/) — `feat:`, `fix:`, `chore:`, `docs:`, `test:`, `refactor:`, `wip:`.
- **No force-unwraps** (`!`) unless justified with a comment.
- **No `print()`** in production code — use `os.Logger` via the `Log` helpers.
- **Concurrency**: keep I/O off the main thread (`async`/background tasks); SwiftData writes happen on the main actor.
- **Tests**: add at least one test per new service. Tests must not depend on the network or on a specific machine's filesystem — use `FileManager.default.temporaryDirectory` and in-memory `ModelContainer`s (see `VibeCodeTrackerAppTests/TestSupport.swift`).

## Architecture

Services parse data sources (JSONL, Git, markdown, package files) off the main actor into plain value types, then upsert into SwiftData on the main actor. Pure logic is separated from I/O so it can be unit-tested. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Pull requests

1. Branch from `main`.
2. Keep PRs focused; write a clear description of what and why.
3. Ensure `xcodebuild test` passes with **no warnings**.
4. Update docs/CHANGELOG when behavior changes.
5. Open the PR using the template.

## Reporting issues

Use the issue templates for bug reports and feature requests. For security-sensitive reports, email the maintainer rather than opening a public issue.
