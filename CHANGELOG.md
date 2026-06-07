# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-06-07

Initial release.

### Added (post-initial)
- Project filters: Claude Code worktrees are detected and hidden from the sidebar
  and KPIs by default (with a "Show worktrees" toggle), and projects are categorized
  by GitHub remote vs local-only via a sidebar filter menu (All / On GitHub / Local only).

### Fixed
- Crash when opening a project that has Git commits: the commit heatmap used a
  descending `chartYScale` domain (`6.5...(-0.5)`), which traps at runtime. Now uses
  an ascending domain with the weekday row inverted, plus a render regression test.

### Added
- Automatic detection of Claude Code projects from `~/.claude/projects/`.
- Schema-tolerant JSONL session parser (tokens, primary model, duration, message
  count, first prompt) with file-level incremental scanning.
- Global dashboard with eight KPIs and a cross-project latest-sessions list.
- Per-project detail view: local KPIs, commit contribution heatmap (Swift Charts),
  recent commits, sessions, backlog, and detected stack.
- Git integration via the `git` CLI with commit-type inference.
- Backlog parser for `TODO.md` / `BACKLOG.md` with P0–P2 priorities.
- Automatic tech-stack detection from marker files and `package.json` dependencies.
- Session status heuristic (in progress / blocked / completed).
- Settings with Keychain-stored Anthropic Admin API key, refresh frequency, and an
  iCloud sync toggle.
- Anthropic usage API client (mock-tested) with token snapshots and estimated costs.
- App icon, dark-mode support, loading/empty states, and an animated refresh.

### Known limitations
- CloudKit sync is scaffolded but inactive (requires an Apple Developer setup and
  removal of unique constraints). See `BLOCKERS.md`.
- Cost figures are estimated from public pricing; the exact usage API schema needs
  validation with a real key. See `BLOCKERS.md`.

[0.1.0]: https://github.com/OWNER/vibecode-traker-app/releases/tag/v0.1.0
