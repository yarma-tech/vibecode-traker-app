import SwiftUI
import Charts

/// One day's commit count in the bar series.
struct CommitBar: Identifiable, Equatable {
    let day: Date     // startOfDay
    let count: Int
    var id: Date { day }
}

/// Pure builder for a daily commit bar series over a trailing window.
enum CommitBars {
    /// One `CommitBar` per day for the trailing `days` days ending today
    /// (inclusive), zero-filled. Days are bucketed by `startOfDay`.
    static func series(commitDates: [Date], now: Date, days: Int = 30, calendar: Calendar = .current) -> [CommitBar] {
        var counts: [Date: Int] = [:]
        for date in commitDates {
            counts[calendar.startOfDay(for: date), default: 0] += 1
        }
        let today = calendar.startOfDay(for: now)
        guard let start = calendar.date(byAdding: .day, value: -(days - 1), to: today) else { return [] }

        var bars: [CommitBar] = []
        for offset in 0..<days {
            guard let day = calendar.date(byAdding: .day, value: offset, to: start) else { continue }
            bars.append(CommitBar(day: day, count: counts[day] ?? 0))
        }
        return bars
    }
}

/// Vertical bar chart of daily commit counts over a trailing window.
struct CommitBarChartView: View {
    let commitDates: [Date]
    var days: Int = 30
    var now: Date = .now

    private var bars: [CommitBar] { CommitBars.series(commitDates: commitDates, now: now, days: days) }

    var body: some View {
        Chart(bars) { bar in
            BarMark(
                x: .value("Day", bar.day, unit: .day),
                y: .value("Commits", bar.count)
            )
            .foregroundStyle(.green)
            .cornerRadius(2)
        }
        .chartYAxis {
            AxisMarks(position: .leading, values: .automatic(desiredCount: 3))
        }
        .chartXAxis {
            AxisMarks(values: .stride(by: .day, count: 7)) { _ in
                AxisGridLine()
                AxisValueLabel(format: .dateTime.day().month(.abbreviated))
            }
        }
        .frame(height: 110)
        .accessibilityLabel("Daily commit bar chart for the last \(days) days")
    }
}

#Preview {
    let cal = Calendar.current
    let today = cal.startOfDay(for: .now)
    let dates: [Date] = (0..<30).flatMap { offset -> [Date] in
        guard offset % 3 == 0, let d = cal.date(byAdding: .day, value: -offset, to: today) else { return [] }
        return Array(repeating: d, count: (offset % 5) + 1)
    }
    return CommitBarChartView(commitDates: dates).padding().frame(width: 520)
}
