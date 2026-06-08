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
- **Costs** — every session is costed automatically from its token counts at published Anthropic list prices. No API key or account required.
- **Local-first** — your data never leaves your Mac; the app makes no network calls.

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

Or install a Release build straight into `/Applications` (it lands in Launchpad):

```bash
./scripts/install.sh          # build Release, install to /Applications, launch
./scripts/install.sh --no-open
```

Or build/test from the command line:

```bash
xcodebuild -scheme VibeCodeTrackerApp -destination 'platform=macOS' build
xcodebuild -scheme VibeCodeTrackerApp -destination 'platform=macOS' test
```

The checked-in `.xcodeproj` means contributors don't need any extra tooling. Maintainers who change the project structure regenerate it from `project.yml` with [XcodeGen](https://github.com/yonaskolb/XcodeGen) — see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## Costs

Every session is costed automatically — token counts × published Anthropic list prices (Opus / Sonnet / Haiku) — on the dashboard and per project. These are **estimates** from public pricing, computed entirely on-device: no API key, no account, and no network calls.

## Privacy

- All project, session, commit, and backlog data is read locally and stored in a local SwiftData database.
- The app makes no network calls — there is no account, login, or API key.
- iCloud sync is opt-in and not active in this version (requires an Apple Developer setup).

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
