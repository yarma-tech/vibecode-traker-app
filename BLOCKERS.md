# Blockers

Items that need Yannick's intervention (access, credentials, decisions).
None are hard-stops for the slices marked Done — each has a documented workaround.

<!--
Template:
## [Slice X] Title
- **Date**:
- **Context**: what I was trying to do
- **Error**: message or symptom
- **Tried**: approaches attempted
- **Need from Yannick**: info / access / decision
- **Impact**: what remains possible / impossible
- **Workaround applied**: what I did to keep going
-->

## [Slice 11] Anthropic Usage API — removed (cost is an on-device estimate)
- **Date**: 2026-06-07 → resolved 2026-06-08
- **Resolution**: The Anthropic usage/key feature was removed entirely. Costs are
  estimated on-device from token counts × published per-MTok list prices
  (`ModelPricing.swift`, Opus/Sonnet/Haiku) and **always display** — no Admin API
  key, no network call, nothing needed from Yannick. The org usage/cost API schema
  is no longer relevant unless org-level billed cost is reintroduced later (it would
  be shown separately from the per-session estimate). See `DECISIONS.md`
  (2026-06-08).

## [Slice 12] CloudKit sync — blocked without Apple Developer setup
- **Date**: 2026-06-07
- **Context**: Multi-machine sync via SwiftData + CloudKit (PRD §10, F4).
- **Blockers** (two, both needing your action):
  1. **Apple Developer account** ($99/yr) + iCloud capability with the
     `iCloud.tech.yannick.vibecodetracker` container + entitlement in Xcode. I do
     not create accounts or sign the app.
  2. **Schema incompatibility**: CloudKit-backed SwiftData forbids
     `@Attribute(.unique)`. V1 uses unique constraints on Project.id/claudeProjectHash,
     Session.sessionId, Commit.sha, etc., and the upsert logic relies on them.
     Enabling CloudKit means removing all `.unique` and de-duplicating manually.
- **Need from Yannick**: decide whether CloudKit is worth the schema rework; if so,
  set up the Apple Developer container + entitlement, then we remove `.unique` and
  switch `PersistenceController.makeContainer(cloudKit: true)`.
- **Impact**: none on local use — the app is fully functional locally.
- **Workaround applied**: graceful degradation (default local store), an inert-but-
  compiling CloudKit code path (`makeContainer(cloudKit:)`, `CloudKitSupport`), and a
  sidebar status indicator that honestly shows "Local only" / "iCloud (setup required)".
