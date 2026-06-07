# Development guide

## Prerequisites

- macOS 14+ and Xcode 15+ (developed with Xcode 26 / Swift 6 toolchain, built in
  Swift 5 language mode).
- `git` CLI (bundled with Xcode).
- [XcodeGen](https://github.com/yonaskolb/XcodeGen) — only needed if you change the
  project structure: `brew install xcodegen`.

## Build & run

```bash
open VibeCodeTrackerApp.xcodeproj      # press ⌘R
# or
xcodebuild -scheme VibeCodeTrackerApp -destination 'platform=macOS' build
```

## Test

```bash
xcodebuild -scheme VibeCodeTrackerApp -destination 'platform=macOS' test CODE_SIGNING_ALLOWED=NO
```

Tests are hermetic: they use `FileManager.default.temporaryDirectory` and in-memory
`ModelContainer`s, and never touch the network or your real `~/.claude`.

## Regenerating the project

`project.yml` is the source of truth for the Xcode project. The generated
`VibeCodeTrackerApp.xcodeproj` is committed so contributors don't need XcodeGen.
After editing `project.yml`:

```bash
xcodegen generate
```

CI also regenerates from `project.yml` before testing.

## Generating the app icon

```bash
swift scripts/generate-app-icon.swift
```

Produces the PNG set + `Contents.json` in
`VibeCodeTrackerApp/Resources/Assets.xcassets/AppIcon.appiconset/`.

## Project layout

```
VibeCodeTrackerApp/
├── App/          App entry, schema, persistence, logging, sync coordinator, prefs
├── Models/       SwiftData @Model types
├── Services/     Parsing / git / API / keychain (I/O off-main, upsert on-main)
├── ViewModels/   Pure KPI calculators + bridges
└── Views/        SwiftUI (Global / Project / Settings / Components)
VibeCodeTrackerAppTests/
├── *Tests.swift  XCTest
├── TestSupport.swift
└── Fixtures/     Sample .jsonl
```

## Adding a new model

1. Create the `@Model` type in `Models/`.
2. Add it to `AppSchema.models`.
3. If it relates to `Project`, add the inverse relationship there.

Because the dev store is recreated on incompatible schema change
(`PersistenceController` wipes + retries), there's no migration to write during
development.

## Gotchas

- **A `ModelContext` does not retain its `ModelContainer`.** Always hold the
  container for as long as a context is used. Test helpers return the container
  (see `TestSupport.makeInMemoryContainer`), never a bare context.
- **No force-unwraps / no `print()`** in app code (use `Log`).
- **Keep I/O off the main actor**; do SwiftData writes on it.
