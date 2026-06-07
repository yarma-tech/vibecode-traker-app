import SwiftUI
import Charts

/// One day cell in the contribution heatmap.
struct HeatmapCell: Identifiable, Equatable {
    let id: Int            // week * 7 + weekday, stable per grid
    let date: Date
    let weekIndex: Int     // column (0 = oldest week)
    let weekday: Int       // row (0 = Sunday)
    let count: Int
}

/// Pure builder for a GitHub-style contribution grid.
enum CommitHeatmap {
    /// Builds cells for the trailing `weeks` weeks ending at `now`, one per day
    /// up to today. Days are bucketed by `startOfDay`.
    static func cells(commitDates: [Date], now: Date, weeks: Int = 13, calendar: Calendar = .current) -> [HeatmapCell] {
        var counts: [Date: Int] = [:]
        for date in commitDates {
            counts[calendar.startOfDay(for: date), default: 0] += 1
        }

        let today = calendar.startOfDay(for: now)
        let weekdayToday = calendar.component(.weekday, from: today) // 1 = Sunday
        guard let thisWeekStart = calendar.date(byAdding: .day, value: -(weekdayToday - 1), to: today),
              let start = calendar.date(byAdding: .weekOfYear, value: -(weeks - 1), to: thisWeekStart)
        else { return [] }

        var cells: [HeatmapCell] = []
        for week in 0..<weeks {
            for weekday in 0..<7 {
                guard let date = calendar.date(byAdding: .day, value: week * 7 + weekday, to: start) else { continue }
                if date > today { continue }
                let count = counts[calendar.startOfDay(for: date)] ?? 0
                cells.append(HeatmapCell(id: week * 7 + weekday, date: date, weekIndex: week, weekday: weekday, count: count))
            }
        }
        return cells
    }

    /// Valid (ascending) y-axis domain for the weekday rows. Must be ascending —
    /// a descending ClosedRange traps at runtime.
    static let weekdayDomain: ClosedRange<Double> = -0.5...6.5

    /// Chart row for a weekday, placing Sunday (0) at the top.
    static func row(forWeekday weekday: Int) -> Int { 6 - weekday }
}

/// GitHub-style commit contribution heatmap rendered with Swift Charts.
struct CommitHeatmapView: View {
    let commitDates: [Date]
    var weeks: Int = 13
    var now: Date = .now

    private var cells: [HeatmapCell] { CommitHeatmap.cells(commitDates: commitDates, now: now, weeks: weeks) }

    var body: some View {
        Chart(cells) { cell in
            RectangleMark(
                x: .value("Week", cell.weekIndex),
                y: .value("Weekday", CommitHeatmap.row(forWeekday: cell.weekday))
            )
            .foregroundStyle(color(for: cell.count))
            .cornerRadius(2)
        }
        .chartXAxis(.hidden)
        .chartYAxis(.hidden)
        .chartXScale(domain: -0.5...(Double(weeks) - 0.5))
        .chartYScale(domain: CommitHeatmap.weekdayDomain) // Sunday on top via row(forWeekday:)
        .chartLegend(.hidden)
        .frame(height: 110)
        .accessibilityLabel("Commit contribution heatmap for the last \(weeks) weeks")
    }

    private func color(for count: Int) -> Color {
        switch count {
        case 0: return Color.gray.opacity(0.12)
        case 1...2: return Color.green.opacity(0.4)
        case 3...5: return Color.green.opacity(0.65)
        default: return Color.green
        }
    }
}
