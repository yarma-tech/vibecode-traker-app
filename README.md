# Vibe Code Tracker

A local macOS dashboard for your [Claude Code](https://www.anthropic.com/claude-code) projects. It reads the sessions already on your machine (`~/.claude/projects/`), cross-references your Git repos and backlogs, and gives you a consolidated view of activity, tokens, and cost — without sending your data anywhere.

> Open the app and know in five seconds where each project stands, what happened this week, how much it cost, and what's blocked.

![CI](https://github.com/OWNER/vibecode-traker-app/actions/workflows/ci.yml/badge.svg)

## Features

- **Auto-detection** — finds every project that has had a Claude Code session, no setup required.
- **Global dashboard** — eight KPIs (active projects, sessions, tokens, cost, averages, top model, blocked) plus the latest sessions across all projects.
- **Project detail** — per-project KPIs, a GitHub-style commit contribution heatmap, recent commits, sessions, backlog, and detected stack.
- **Session insight** — token usage, primary model, duration, and an inferred status (in progress / blocked / completed).
- **Git integration** — recent commits with inferred type (feat/fix/…) via the `git` CLI.
- **Backlog** — parses `TODO.md` / `BACKLOG.md` (with P0–P2 priorities).
- **Stack detection** — infers TypeScript, Rust, Python, Next.js, Supabase, Twilio, and more.
- **Costs (optional)** — add an Anthropic Admin API key to see token/cost data; everything works without it.
- **Local-first** — your data never leaves your Mac (except the Anthropic API, only if you opt in).

## Requirements

- macOS 14 (Sonoma) or later
- Xcode 15 or later (developed with Xcode 26)
- The `git` command-line tools (ships with Xcode)

## Install

This is a source build for now (a signed DMG / Homebrew cask may come later).

```bash
git clone https://github.com/OWNER/vibecode-traker-app.git
cd vibecode-traker-app
open VibeCodeTrackerApp.xcodeproj   # then press Run (⌘R)
```

Or build/test from the command line:

```bash
xcodebuild -scheme VibeCodeTrackerApp -destination 'platform=macOS' build
xcodebuild -scheme VibeCodeTrackerApp -destination 'platform=macOS' test
```

The checked-in `.xcodeproj` means contributors don't need any extra tooling. Maintainers who change the project structure regenerate it from `project.yml` with [XcodeGen](https://github.com/yonaskolb/XcodeGen) — see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## Setup (optional: costs)

By default the app runs in tokens-only mode and shows cost as "—".

To see cost data: open **Settings** in the sidebar, paste your **Anthropic Admin API key**, and press **Save**. The key is stored only in your macOS Keychain and is used solely for `api.anthropic.com`.

> Note: the exact usage/cost API schema is still being validated; current cost figures are derived from public list prices applied to token counts. See [BLOCKERS.md](BLOCKERS.md).

## Privacy

- All project, session, commit, and backlog data is read locally and stored in a local SwiftData database.
- The **only** network destination is `api.anthropic.com`, and only if you add an API key.
- iCloud sync is opt-in and not active in this version (requires an Apple Developer setup).
- Your API key lives exclusively in the macOS Keychain — never in files, logs, or sync.

## Documentation

- [Architecture](docs/ARCHITECTURE.md) — layers, data flow, models
- [Development guide](docs/DEVELOPMENT.md) — build, test, project layout
- [Product requirements](docs/PRD.md)
- [Architectural decisions](DECISIONS.md) · [Known blockers](BLOCKERS.md)

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) and our [Code of Conduct](CODE_OF_CONDUCT.md).

## License

[MIT](LICENSE) © 2026 Yannick Maillard.

Vibe Code Tracker is compatible with Claude Code but is not affiliated with or endorsed by Anthropic. "Claude" is a trademark of Anthropic.
