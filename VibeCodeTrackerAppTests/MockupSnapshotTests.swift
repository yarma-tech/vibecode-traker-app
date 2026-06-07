import XCTest
import SwiftUI
import SwiftData
@testable import VibeCodeTrackerApp

/// Phase-0 visual gate: renders the redesigned components to PNGs under
/// build/mockups/ for review. Not a behavioural test — it asserts only that
/// rendering succeeds. Remove (or keep as a regression guard) per plan Task 1.13.
@MainActor
final class MockupSnapshotTests: XCTestCase {

    func testRenderMockups() throws {
        // Retain the container for the whole test: a context whose container has
        // deallocated traps inside SwiftData (see TestSupport.makeInMemoryContainer).
        let container = try TestSupport.makeInMemoryContainer()
        let sessions = Sample.makeSessions(in: container.mainContext)

        try snapshot(MockGallery.dashboard(sessions: sessions), name: "01-dashboard", size: CGSize(width: 760, height: 560))
        try snapshot(MockGallery.projectDetail(sessions: sessions), name: "02-project-detail", size: CGSize(width: 760, height: 820))
        try snapshot(MockGallery.sessionsTable(sessions: sessions), name: "03-sessions-table", size: CGSize(width: 720, height: 260))
        try snapshot(MockGallery.commitBars(dates: Sample.commitDates), name: "04-commit-bars", size: CGSize(width: 720, height: 180))
    }

    private func snapshot<V: View>(_ view: V, name: String, size: CGSize) throws {
        let renderer = ImageRenderer(content:
            view.frame(width: size.width, alignment: .topLeading)
                .padding(Spacing.lg)
                .background(Color(nsColor: .windowBackgroundColor))
        )
        renderer.scale = 2
        let image = try XCTUnwrap(renderer.nsImage, "render \(name) produced no image")
        let tiff = try XCTUnwrap(image.tiffRepresentation)
        let png = try XCTUnwrap(NSBitmapImageRep(data: tiff)?.representation(using: .png, properties: [:]))
        let dir = Self.outputDir()
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let url = dir.appendingPathComponent("\(name).png")
        try png.write(to: url)
        print("MOCKUP_WRITTEN \(url.path)")
    }

    /// build/mockups/ at the repo root, derived from this file's compile-time path.
    static func outputDir() -> URL {
        URL(fileURLWithPath: #filePath)            // .../VibeCodeTrackerAppTests/MockupSnapshotTests.swift
            .deletingLastPathComponent()           // VibeCodeTrackerAppTests
            .deletingLastPathComponent()           // repo root
            .appendingPathComponent("build/mockups")
    }
}

// MARK: - Sample data (throwaway)

private enum Sample {
    static let now = Date()

    /// Builds sample sessions and inserts them into `ctx`. The caller MUST retain
    /// the container backing `ctx` (see TestSupport.makeInMemoryContainer) — using
    /// a context whose container has deallocated traps inside SwiftData.
    @MainActor
    static func makeSessions(in ctx: ModelContext) -> [Session] {
        let specs: [(String, String, Int, Int, SessionStatus, TimeInterval)] = [
            ("Refactor the sync coordinator to use structured concurrency and cancellation", "Opus", 182_000, 5_400, .inProgress, 3_600),
            ("Fix crash opening a project with commits", "Sonnet", 64_000, 1_800, .completed, 86_400),
            ("Add CloudKit support behind a preference", "Sonnet", 121_000, 4_200, .blocked, 200_000),
            ("Write the README and contribution docs", "Haiku", 28_000, 900, .completed, 300_000),
        ]
        return specs.map { prompt, family, tokens, dur, status, ago in
            let s = Session(sessionId: UUID().uuidString,
                            startedAt: now.addingTimeInterval(-ago),
                            durationSeconds: dur,
                            modelFamily: family,
                            status: status.rawValue,
                            firstUserPrompt: prompt)
            s.inputTokens = tokens / 2          // distribute so totalTokens == tokens
            s.outputTokens = tokens - tokens / 2
            ctx.insert(s)
            return s
        }
    }

    static let commitDates: [Date] = {
        let cal = Calendar.current
        let today = cal.startOfDay(for: now)
        return (0..<30).flatMap { offset -> [Date] in
            guard offset % 2 == 0, let d = cal.date(byAdding: .day, value: -offset, to: today) else { return [] }
            return Array(repeating: d, count: (offset % 4) + 1)
        }
    }()

    static let stack = ["Swift", "SwiftUI", "SwiftData", "Swift Charts", "XcodeGen"]
}

// MARK: - Mock screens (reuse the REAL components)

private enum MockGallery {

    static func dashboard(sessions: [Session]) -> some View {
        VStack(alignment: .leading, spacing: Spacing.lg) {
            // Hero tier
            HStack(spacing: Spacing.md) {
                HeroKPI(title: "Sessions", value: "12", caption: "this week", systemImage: "bubble.left.and.bubble.right")
                HeroKPI(title: "Cost", value: "$48.20", caption: "est. · this week", systemImage: "dollarsign.circle")
                HeroKPI(title: "Blocked", value: "1", caption: "sessions", systemImage: "exclamationmark.triangle")
            }
            // Secondary strip
            Card {
                HStack(spacing: 0) {
                    SecondaryMetric(label: "Projects", value: "7")
                    Divider().frame(height: 28)
                    SecondaryMetric(label: "Tokens", value: "1.2M")
                    Divider().frame(height: 28)
                    SecondaryMetric(label: "Avg/session", value: "96K")
                    Divider().frame(height: 28)
                    SecondaryMetric(label: "Avg time", value: "42m")
                    Divider().frame(height: 28)
                    SecondaryMetric(label: "Top model", value: "Opus")
                }
            }
            SectionHeader(title: "Latest sessions across all projects")
            sessionsTable(sessions: sessions)
        }
    }

    static func projectDetail(sessions: [Session]) -> some View {
        VStack(alignment: .leading, spacing: Spacing.lg) {
            VStack(alignment: .leading, spacing: Spacing.xs) {
                Text("vibecode-tracker-app").font(.largeTitle.weight(.semibold))
                Text("~/Developer/vibecode-traker-app").font(.callout).foregroundStyle(.secondary)
                DisclosureGroup {
                    StackTagsView(stack: Sample.stack).padding(.top, Spacing.xs)
                } label: {
                    Text("Stack").font(.subheadline.weight(.medium))
                }
                .padding(.top, Spacing.xs)
            }
            HStack(spacing: Spacing.md) {
                HeroKPI(title: "Sessions", value: "34", caption: nil, systemImage: "bubble.left.and.bubble.right")
                HeroKPI(title: "Tokens", value: "1.0M", caption: nil, systemImage: "number")
                HeroKPI(title: "Cost", value: "$31.90", caption: "est.", systemImage: "dollarsign.circle")
                HeroKPI(title: "Avg time", value: "38m", caption: nil, systemImage: "clock")
            }
            VStack(alignment: .leading, spacing: Spacing.sm) {
                SectionHeader(title: "Commit activity — last 30 days")
                commitBars(dates: Sample.commitDates)
            }
            VStack(alignment: .leading, spacing: Spacing.sm) {
                SectionHeader(title: "Recent commits", count: 10, isExpanded: .constant(true))
            }
            VStack(alignment: .leading, spacing: Spacing.sm) {
                SectionHeader(title: "Recent sessions", count: 4, isExpanded: .constant(true))
                sessionsTable(sessions: sessions)
            }
            SectionHeader(title: "Backlog", count: 0, isExpanded: .constant(false))
        }
    }

    static func sessionsTable(sessions: [Session]) -> some View { Card { SessionsTableView(sessions: sessions) } }
    static func commitBars(dates: [Date]) -> some View { Card { CommitBarChartView(commitDates: dates) } }
}

// MARK: - Tier helpers (promoted to real views in Phase 1 — see Task 1.5)

private struct HeroKPI: View {
    let title: String; let value: String; var caption: String?; let systemImage: String
    var body: some View {
        Card {
            VStack(alignment: .leading, spacing: Spacing.sm) {
                Label(title.uppercased(), systemImage: systemImage)
                    .font(.caption2.weight(.semibold)).foregroundStyle(.secondary)
                    .labelStyle(.titleAndIcon)
                Text(value).font(.system(.largeTitle, design: .rounded).weight(.semibold))
                    .lineLimit(1).minimumScaleFactor(0.6)
                if let caption { Text(caption).font(.caption2).foregroundStyle(.secondary) }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}

private struct SecondaryMetric: View {
    let label: String; let value: String
    var body: some View {
        VStack(spacing: 2) {
            Text(value).font(.headline.weight(.semibold)).monospacedDigit()
            Text(label.uppercased()).font(.caption2).foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity)
    }
}
