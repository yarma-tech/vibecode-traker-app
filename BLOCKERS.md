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

## [Slice 11] Anthropic Usage API — schema assumed, needs validation with a real key
- **Date**: 2026-06-07
- **Context**: Implementing `AnthropicUsageClient` for
  `GET /v1/organizations/usage_report/messages` (headers `x-api-key`,
  `anthropic-version: 2023-06-01`).
- **Constraint honored**: I made **no** real network calls — there is no Admin key
  to test with, and the app never calls the API unless you store a real key in the
  Keychain (Settings). All tests use a mocked `URLProtocol`.
- **Assumption**: response shape is
  `{ "data": [ { "starting_at", "ending_at", "results": [ { "model",
  "input_tokens", "output_tokens", "cache_read_input_tokens",
  "cache_creation_input_tokens" } ] } ] }`. The parser is tolerant (also accepts
  tokens directly on each `data` entry) and ignores unknown fields.
- **Cost**: the messages usage report returns **tokens, not USD**. V1 derives cost
  from public per-MTok list prices in `ModelPricing.swift` (Opus/Sonnet/Haiku) and
  prices each local session. This is an **estimate**.
- **Need from Yannick**:
  1. Add a real Admin API key in Settings and click "Test connection" / Refresh.
  2. Confirm the JSON shape matches the assumption (adjust `AnthropicUsageClient.parse`
     if not) — check Console logs (category `network`).
  3. Decide whether to switch to the authoritative `/v1/organizations/cost_report`
     endpoint for exact USD instead of the local price estimate.
- **Impact**: Without a key the app runs fully in tokens-only mode (cost shows "—").
  With a key, token snapshots + estimated costs populate.
- **Workaround applied**: tolerant parser + documented price table; mocked tests.

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
