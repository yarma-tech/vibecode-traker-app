# Architecture

Vibe Code Tracker is a local-first macOS app: Swift + SwiftUI + SwiftData. It
makes no network calls — everything is derived on-device.

## Layers

```
┌────────────────────────────────────────────────────────────┐
│ Data sources (read-only, on disk)                          │
│  ~/.claude/projects/<hash>/*.jsonl   sessions              │
│  <project>/.git                      commits (git CLI)     │
│  <project>/TODO.md, BACKLOG.md       backlog               │
│  <project>/package.json, Cargo.toml… stack signals         │
└────────────────────────────────────────────────────────────┘
                         │  parse off the main actor → value types
                         ▼
┌────────────────────────────────────────────────────────────┐
│ Services                                                   │
│  ClaudeProjectsScanner · JSONLParser · SessionSyncService  │
│  GitInspector · GitSyncService · BacklogParser/Sync        │
│  StackDetector/Sync · StatusSyncService                    │
└────────────────────────────────────────────────────────────┘
                         │  upsert on the main actor
                         ▼
┌────────────────────────────────────────────────────────────┐
│ SwiftData store                                            │
│  Project · Session · Commit · BacklogItem · FileSyncState  │
└────────────────────────────────────────────────────────────┘
                         │  @Query / observation
                         ▼
┌────────────────────────────────────────────────────────────┐
│ SwiftUI views                                              │
│  ContentView (sidebar + detail router)                     │
│  GlobalDashboardView · ProjectDetailView · SettingsView    │
└────────────────────────────────────────────────────────────┘
```

## Core pattern: parse off-main, upsert on-main

Each service splits I/O + CPU from persistence:

1. A pure/`nonisolated` step reads the filesystem (or network) and returns plain
   `Sendable` value types (`DiscoveredProject`, `ParsedSession`, `GitCommit`, …).
   This runs in a `Task.detached` so the main thread stays responsive.
2. A `@MainActor apply(...)` step upserts those values into SwiftData.

This keeps pure logic unit-testable without SwiftData and keeps all `ModelContext`
access on the main actor.

## The sync pipeline

`SyncCoordinator.fullSync` runs the services in dependency order:

1. `ClaudeProjectsScanner` — discover projects (provisional paths).
2. `SessionSyncService` — parse JSONL, recover real paths from `cwd`.
3. `GitSyncService` — read commits (needs real paths).
4. `BacklogSyncService` — parse TODO/BACKLOG.
5. `StackSyncService` — detect stack.
6. `StatusSyncService` — recompute session status (needs commits).

It runs on launch, on the Refresh button, and on a background timer (the
configurable refresh frequency).

## Models

`Project` owns cascading relationships to `Session`, `Commit`, and `BacklogItem`.
`FileSyncState` tracks per-file scan state (local, never synced). All models are
registered once in `AppSchema.models`.

## Notable decisions

See [DECISIONS.md](../DECISIONS.md) — highlights:

- **XcodeGen generates a committed `.xcodeproj`** (contributors need no tooling).
- **Swift 5 language mode** to avoid Swift 6 strict-concurrency churn with SwiftData.
- **Not sandboxed** in V1 (needs `~/.claude` access + `git`).
- **Paths come from JSONL `cwd`**, not the lossy folder-name decode.
- A `ModelContext` does not retain its container — tests must keep the container alive.
