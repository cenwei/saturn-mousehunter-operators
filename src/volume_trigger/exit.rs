//! Volume trigger exit matching — reverse signal exit + EOD fallback.
//!
//! Reference: `volume_trigger_strategy.py` exit logic
//!
//! # Audit (no look-ahead bias)
//! - Reverse exit: `bidx > entry.bar_idx` ensures exit is after entry (✅)
//! - T+1 enforcement: same-day exit blocked when t_plus_one is true (✅)
//! - EOD fallback: uses last bar index which is only known at end of data (correct for backtest) (✅)

use crate::traits::Bar;
use crate::volume_trigger::signal::VolumeTriggerSignal;

/// Matched exit information.
#[derive(Debug, Clone)]
pub struct MatchedExit {
    /// Global bar index of the exit bar
    pub exit_bar_idx: usize,
    /// Timestamp of the exit bar
    pub exit_ts: String,
    /// Exit price (close of exit bar)
    pub exit_price: f64,
    /// Exit reason: "volume_trigger_reverse_short" or "volume_trigger_eod"
    pub exit_reason: String,
    /// Number of bars held
    pub holding_bars: usize,
}

/// Match a long entry signal to its exit.
///
/// Strategy:
/// 1. Find the next `volume_trigger_short` for the same symbol AFTER the entry bar
/// 2. T+1 enforcement: if t_plus_one, exit must be on a different trading day
/// 3. If no matching short signal found, fall back to EOD (close at last bar)
///
/// Returns `None` only if no valid exit can be found (e.g., entry is at the very last bar).
pub fn match_volume_trigger_exit(
    entry: &VolumeTriggerSignal,
    exit_signals: &[VolumeTriggerSignal],
    bars: &[Bar],
    indices: &[usize], // per-symbol bar index → global bar index
    t_plus_one: bool,
) -> Option<MatchedExit> {
    // Try reverse short signal exit first
    for exit_signal in exit_signals {
        // Must be after the entry bar
        if exit_signal.bar_idx <= entry.bar_idx {
            continue;
        }

        // T+1: exit must be on a different trading day
        if t_plus_one && exit_signal.trading_day == entry.trading_day {
            continue;
        }

        // Same-day non-T+1: exit must be on same day or later
        if !t_plus_one && exit_signal.trading_day < entry.trading_day {
            continue;
        }

        let exit_global_idx = indices[exit_signal.bar_idx];
        let holding_bars = exit_signal.bar_idx.saturating_sub(entry.bar_idx);

        return Some(MatchedExit {
            exit_bar_idx: exit_global_idx,
            exit_ts: exit_signal.signal_ts.clone(),
            exit_price: exit_signal.close,
            exit_reason: "volume_trigger_reverse_short".to_string(),
            holding_bars: holding_bars.max(1),
        });
    }

    // Fallback: hold until end of data (EOD)
    let last_local_idx = indices.len().saturating_sub(1);
    if last_local_idx > entry.bar_idx {
        let last_global_idx = indices[last_local_idx];
        let holding_bars = last_local_idx.saturating_sub(entry.bar_idx);
        Some(MatchedExit {
            exit_bar_idx: last_global_idx,
            exit_ts: bars[last_global_idx].ts_utc.clone(),
            exit_price: bars[last_global_idx].close,
            exit_reason: "volume_trigger_eod".to_string(),
            holding_bars: holding_bars.max(1),
        })
    } else {
        // Entry at the very last bar — cannot exit
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::volume_trigger::signal::VolumeTriggerSignalType;
    use std::collections::BTreeMap;

    fn make_bar(symbol: &str, ts: &str, close: f64, volume: f64) -> Bar {
        Bar {
            symbol: symbol.to_string(),
            ts_utc: ts.to_string(),
            open: close - 0.1,
            high: close + 0.2,
            low: close - 0.3,
            close,
            volume,
            extra_columns: BTreeMap::new(),
        }
    }
    #[test]
    fn reverse_short_exit_matches_after_entry() {
        let entry = VolumeTriggerSignal {
            symbol: "TEST".into(),
            bar_idx: 0,
            signal_ts: "2026-01-05T02:00:00Z".into(),
            trading_day: "2026-01-05".into(),
            close: 10.0,
            signal_type: VolumeTriggerSignalType::Long,
        };

        let exit = VolumeTriggerSignal {
            symbol: "TEST".into(),
            bar_idx: 1,
            signal_ts: "2026-01-06T02:00:00Z".into(),
            trading_day: "2026-01-06".into(),
            close: 11.0,
            signal_type: VolumeTriggerSignalType::Short,
        };

        let bars = vec![
            make_bar("TEST", "2026-01-05T02:00:00Z", 10.0, 100.0),
            make_bar("TEST", "2026-01-06T02:00:00Z", 11.0, 200.0),
        ];
        let indices: Vec<usize> = vec![0, 1];

        let result = match_volume_trigger_exit(&entry, &[exit], &bars, &indices, true);
        assert!(result.is_some());
        let matched = result.unwrap();
        assert_eq!(matched.exit_reason, "volume_trigger_reverse_short");
        assert_eq!(matched.exit_price, 11.0);
    }

    #[test]
    fn t_plus_one_blocks_same_day_exit() {
        let entry = VolumeTriggerSignal {
            symbol: "TEST".into(),
            bar_idx: 0,
            signal_ts: "2026-01-05T02:00:00Z".into(),
            trading_day: "2026-01-05".into(),
            close: 10.0,
            signal_type: VolumeTriggerSignalType::Long,
        };

        let exit_same_day = VolumeTriggerSignal {
            symbol: "TEST".into(),
            bar_idx: 1,
            signal_ts: "2026-01-05T05:00:00Z".into(),
            trading_day: "2026-01-05".into(), // same day, blocked by T+1
            close: 11.0,
            signal_type: VolumeTriggerSignalType::Short,
        };

        let bars = vec![
            make_bar("TEST", "2026-01-05T02:00:00Z", 10.0, 100.0),
            make_bar("TEST", "2026-01-05T05:00:00Z", 11.0, 200.0),
            make_bar("TEST", "2026-01-06T01:00:00Z", 12.0, 150.0),
        ];
        let indices: Vec<usize> = vec![0, 1, 2];

        // T+1: same-day exit should be blocked → falls back to EOD (last bar)
        let result = match_volume_trigger_exit(&entry, &[exit_same_day], &bars, &indices, true);
        assert!(result.is_some());
        let matched = result.unwrap();
        assert_eq!(matched.exit_reason, "volume_trigger_eod");
    }

    #[test]
    fn eod_fallback_when_no_short_signal() {
        let entry = VolumeTriggerSignal {
            symbol: "TEST".into(),
            bar_idx: 0,
            signal_ts: "2026-01-05T01:00:00Z".into(),
            trading_day: "2026-01-05".into(),
            close: 10.0,
            signal_type: VolumeTriggerSignalType::Long,
        };

        let bars = vec![
            make_bar("TEST", "2026-01-05T01:00:00Z", 10.0, 100.0),
            make_bar("TEST", "2026-01-06T01:00:00Z", 11.0, 150.0),
            make_bar("TEST", "2026-01-07T01:00:00Z", 12.0, 200.0),
        ];
        let indices: Vec<usize> = vec![0, 1, 2];

        // No exit signals at all → EOD
        let result = match_volume_trigger_exit(&entry, &[], &bars, &indices, true);
        assert!(result.is_some());
        let matched = result.unwrap();
        assert_eq!(matched.exit_reason, "volume_trigger_eod");
        assert_eq!(matched.exit_price, 12.0); // last bar close
    }
}
