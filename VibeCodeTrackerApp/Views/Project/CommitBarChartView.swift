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
