# UI Clarity Pass — Design Spec

**Date:** 2026-06-07
**Status:** Proposed
**Scope:** Global Dashboard + Project Detail views (SwiftUI, macOS)

## 1. Problem

The app is functional and visually clean, but two screens have a flat visual
hierarchy — everything carries the same weight and is expanded at once:

- **Global Dashboard:** eight KPI cards of identical size. The eye has no anchor,
  so the metrics that drive decisions (activity, spend, what's blocked) don't
  stand out from context metrics (top model, averages).
- **Project Detail:** a single long scroll of five equal-weight sections
  (commit heatmap, recent commits, stack, recent sessions, backlog), all open
  simultaneously, producing a dense, scroll-heavy page.

Both screens also re-implement the same card chrome (rounded `.background.secondary`
+ `.separator` border, radius 12) inline, which invites visual drift over time.

The user's chosen goal is **UX clarity** across the whole app (not a pure aesthetic
restyle and not a performance pass), with these two screens as the priority.

## 2. Goals / Non-Goals

**Goals**
- Establish a clear two-tier hierarchy on the dashboard so the three decision
  metrics dominate.
- Reduce project-detail density via progressive disclosure (collapsible sections,
  empty-state compaction, importance ordering).
- Make the headline Cost metric meaningful without an API key, by deriving it from
  token counts at published Anthropic list prices.
- Replace the commit heatmap with a vertical bar chart of daily commit activity.
- Turn the session list into a compact, column-aligned table.
- Move the stack into a collapsed disclosure under the project name.
- Extract two shared primitives (`Card`, `SectionHeader`) and a small spacing
  scale so both screens stay consistent.

**Non-Goals**
- No changes to the sidebar, settings, or sync logic.
- No SwiftData schema / model migration.
- No performance/rendering work.
- No new persisted metrics. The Anthropic-actual cost plumbing stays in place; it
  simply stops being the headline Cost value.
- No segmented-tab restructuring of the detail view (single scroll is retained).

## 3. Approach

Incremental, native-first ("Approach A"): keep the single-scroll layouts, introduce
hierarchy and progressive disclosure, and back the shared chrome with two small
reusable primitives. The only logic change is the cost derivation; everything else
is view-layer.

## 4. Detailed Design

### 4.1 Shared primitives (new — `Views/Components/`)

- **`Card<Content: View>`** — wraps content in the existing rounded
  `.background.secondary` fill + `.separator` 0.5pt border at radius 12. Replaces
  the inline chrome currently duplicated in `KPICard` and `LatestSessionsList`.
- **`SectionHeader`** — a `.headline` title with an optional trailing count badge,
  an optional leading collapse chevron, and an optional trailing action slot. Used
  by the detail sections and the dashboard "Latest sessions" header.
- **`Spacing`** — a tiny enum of named steps (`xs = 4, sm = 8, md = 12, lg = 20`)
  to replace ad-hoc magic numbers in the touched views. Intentionally minimal; not
  a full token system.

These are pure presentational views with no data dependencies, independently
previewable.

### 4.2 Estimated cost (logic change)

**Intent:** Cost should answer "what would this cost at Anthropic's real API list
price?" — always available, no API key required.

- `ModelPricing.cost(family:inputTokens:outputTokens:cacheReadTokens:cacheCreationTokens:)`
  already exists and prices exactly the fields `Session` stores. No new pricing
  code is needed.
- Add `estimatedCostUSD: Double` to `SessionStat`, computed in
  `GlobalDashboardViewModel.stats(from:)` where the full `Session` (and thus the
  token breakdown) is available.
- `DashboardCalculator` sums `estimatedCostUSD` for the week into a non-optional
  `costThisWeek: Double`. `ProjectMetrics` sums it into a non-optional
  `totalCostUSD: Double`.
- The Cost KPI always renders a dollar value (the `—` / "configure API key" state
  is removed). Caption becomes `est. · this week`; tooltip: "Estimated at published
  Anthropic list prices (see ModelPricing)."
- `Session.totalCostUSD` (Anthropic-actual) is left in the model and parsing path
  untouched; it is simply no longer the source for the headline KPI.
- **Unknown model family:** `ModelPricing.price(family:)` returns zero prices for
  families outside `{Opus, Sonnet, Haiku}` (e.g. `Session`'s `"Unknown"`). Such a
  session contributes `$0.00` — expected, not a bug. The summed cost is still
  non-optional, so the "always renders a value" criterion holds.

**Boundary:** the calculator stays pure and Sendable. Cost is derived once in
`GlobalDashboardViewModel.stats(from:)` from the full `Session` (which has the
per-category token breakdown) and *stored* on the stat as `estimatedCostUSD`; the
calculators only sum that stored field — they do not re-derive cost from token
counts (`SessionStat` carries `totalTokens`, not the breakdown). Unit tests
therefore construct stats with `estimatedCostUSD` set directly.

### 4.3 Dashboard KPI tiering

- **Hero row (3 large `Card`s):** Sessions (this week), Cost (this week, estimated),
  Blocked (sessions). These map to the product's "five-second" questions: how much
  happened, what it costs, what needs attention.
- **Secondary strip (compact, lighter weight):** Projects (active), Tokens (week),
  Avg/session, Avg time, Top model. Rendered as small label/value cells rather than
  full cards, to visually de-emphasize them relative to the hero row.
- All eight metrics remain present; only their visual weight changes.

### 4.4 Commit activity → vertical bar chart

- Replace `CommitHeatmapView` (and the `CommitHeatmap.cells` grid builder) with a
  vertical bar chart of **daily commit counts over the last 30 days**.
- New pure builder, e.g. `CommitBars.series(commitDates:now:days:calendar:)`,
  returning one `(date, count)` per day in the window (zero-filled), kept pure for
  unit testing.
- Rendered with Swift Charts `BarMark(x: .value("Day", date, unit: .day),
  y: .value("Commits", count))`, green fill, month/day ticks on the X axis, a
  hidden or minimal Y axis, ~110pt height (matching today's footprint).
- Section heading updates from "last 90 days" to "last 30 days".
- The old heatmap builder's tests are retired; the new bar-series builder gets
  equivalent coverage (windowing, zero-fill, bucketing-by-day).

### 4.5 Recent sessions → compact table

- Replace the two-line `SessionRow` with a single-line, column-aligned row using a
  `Grid` (true column alignment) plus a faint header row.
- Columns: **status** (badge/icon) · **prompt preview (≤ 40 chars, truncated)** ·
  **model** · **tokens** · **duration** · **date (relative)**.
- No per-session cost column (kept deliberately simple).
- The faint column-header row lives *inside* the section's disclosure body (shown
  when the Recent sessions section is expanded, per 4.7) — it is not a second
  always-visible header alongside `SectionHeader`.
- Empty state retains the existing `ContentUnavailableView`.

### 4.6 Stack → disclosure under project name

- Remove the standalone "Stack" detail section.
- In the project-detail header, place a collapsed `DisclosureGroup` labeled `Stack`
  (optionally with a count) directly under the project name. Expanding it reveals
  the existing `StackTagsView`. Collapsed by default.

### 4.7 Detail section progressive disclosure

- Remaining sections become collapsible via `SectionHeader` + `DisclosureGroup`,
  each showing a **count** in its header (e.g. `Recent commits (10)`,
  `Backlog (3)`).
- **Default open:** Commit activity, Recent sessions. **Default collapsed:** Recent
  commits, Backlog. Open/closed state persisted in `AppStorage` (global keys, not
  per-project, to stay simple). New keys are declared in `PreferenceKey`
  (`App/Preferences.swift`), the established home for storage keys — not inlined as
  raw strings in the view.
- **Empty sections** collapse to a single muted line rather than a large blank
  block (e.g. "No backlog items").
- **Order by importance:** Commit activity → Recent sessions → Recent commits →
  Backlog. Sessions lead the lists (this is a Claude Code session tracker), so
  Recent sessions sits above Recent commits. (Stack now lives in the header per 4.6.)

## 5. Components & Boundaries

| Unit | Responsibility | Depends on |
|------|----------------|-----------|
| `Card` | Reusable card chrome | — |
| `SectionHeader` | Title + count + collapse affordance | — |
| `Spacing` | Named spacing constants | — |
| `CommitBars` (pure) | Daily commit series builder | Foundation only |
| `CommitBarChartView` | Renders the bar chart | `CommitBars`, Charts |
| `SessionsTableView` | Column-aligned session rows | `Session`, `Format` |
| `GlobalDashboardViewModel` | Adds `estimatedCostUSD` to stats | `ModelPricing` |
| `DashboardCalculator` / `ProjectMetrics` | Sum estimated cost | `SessionStat` |
| `GlobalDashboardView` | Hero + secondary KPI tiers | `Card`, viewmodel |
| `ProjectDetailView` | Header w/ stack disclosure, collapsible sections | the above |

## 6. Testing

- **Estimated cost:** unit-test `DashboardCalculator` and `ProjectMetrics` for the
  new non-optional cost summed from `estimatedCostUSD`; update existing tests that
  asserted `costThisWeek == nil` when no API cost was present.
- **Commit bars:** unit-test `CommitBars.series` for window length, zero-fill of
  empty days, and per-day bucketing (mirroring the retired heatmap tests).
- **Views:** keep/refresh SwiftUI previews for `Card`, `SectionHeader`,
  `CommitBarChartView`, `SessionsTableView`, and the two screens.
- Full test suite must stay green; only cost/heatmap-related tests are expected to
  change.

## 7. Risks & Mitigations

- **Estimated cost is an approximation.** `ModelPricing` notes prices are published
  list estimates (see BLOCKERS.md). Mitigation: the caption/tooltip label it as
  "estimated", so it is never presented as billed truth.
- **Daily 30-day bars on narrow windows.** ~30 thin bars could feel dense on small
  windows. Mitigation: fixed minimal bar width and chart height; weekly bucketing
  remains an easy future fallback if needed.
- **Persisted collapse state surprising users.** Mitigation: sensible defaults
  (activity/commits open) and global (not per-project) keys keep behavior
  predictable.

## 8. Success Criteria

- Dashboard presents **three** visually dominant elements, not eight equal ones,
  while still showing all metrics.
- Project-detail initial viewport shows header + KPIs + commit activity without a
  long scroll; heavy lists sit behind disclosure.
- Cost shows a real value with no API key configured.
- Commit activity is a vertical daily bar chart (30 days).
- Sessions render as an aligned table with a ≤40-char prompt preview.
- Stack is hidden under the project name until expanded.
- `Card` and `SectionHeader` are the single source of card/section chrome on both
  screens.

## 9. Implementation Phasing

### Phase 0 — High-fidelity mockups (approval gate before build)

The redesign must be visually approved before real-data wiring begins.

- Build the redesigned pieces as SwiftUI views driven by **mock/sample data** with
  `#Preview`s: `Card`, `SectionHeader`, the hero + secondary KPI tiers,
  `CommitBarChartView`, `SessionsTableView`, the stack disclosure header, and the
  collapsible detail sections — plus an assembled mock of each full screen
  (dashboard, project detail).
- Render each preview to **PNG via an `ImageRenderer`-based snapshot harness** (a
  small XCTest in the test target) into a known output directory.
- Share the screenshots with the user; iterate on layout/hierarchy/spacing until
  approved.
- Caveat: `ImageRenderer` rasterizes without window vibrancy, so system materials
  (`.background.secondary`, `.bar`) may render marginally flatter than in the live
  window. This is acceptable for validating hierarchy and layout; final polish is
  verified in-app during implementation.
- No cost-calc change, no viewmodel/data wiring, no heatmap removal in this phase —
  mock data only, so the previews are throwaway-safe yet reused as the real
  component bodies once approved.

### Phase 1+ — Implementation (only after Phase 0 approval)

Wire the approved components into the real views and data, then layer in the logic
changes: estimated-cost derivation (+ tests), `CommitBars` builder (+ tests) and
heatmap retirement, detail collapsibility with `PreferenceKey` storage keys, and
the dashboard KPI tiering. Full test suite stays green. The detailed step ordering
is produced by the implementation plan.
