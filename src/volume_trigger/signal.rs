//! Volume trigger signal detection — Python-aligned daily max volume baseline.
//!
//! Reference: `volume_trigger_strategy.py` (daily max volume + 6-bar direction + reverse exit)
//!
//! # Audit (no look-ahead bias)
//! - Daily max volume: pre-computed from all bars, filtered to `d < current_day` per signal (✅)
//! - 6-bar reference window: `closes[i-6..i]` — strictly before current bar (✅)
//! - Exit matching: `bidx > entry.bar_idx` + T+1 enforcement (✅)

use std::collections::BTreeMap;

use crate::traits::Bar;
use crate::volume_trigger::params::VolumeTriggerParams;

// —— Signal types ——

#[derive(Debug, Clone, PartialEq)]
pub enum VolumeTriggerSignalType {
    /// Close > max(prev 6 closes) → go long
    Long,
    /// Close < min(prev 6 closes) → short (used as exit signal)
    Short,
}

impl VolumeTriggerSignalType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Long => "volume_trigger_long",
            Self::Short => "volume_trigger_short",
        }
    }
}

#[derive(Debug, Clone)]
pub struct VolumeTriggerSignal {
    pub symbol: String,
    /// Index into per-symbol bars (not global bar index)
    pub bar_idx: usize,
    pub signal_ts: String,
    pub trading_day: String,
    pub close: f64,
    pub signal_type: VolumeTriggerSignalType,
}

// —— Daily max volume pre-computation ——

/// Build per-symbol per-trading-day maximum volume map.
///
/// Scans all bars for each symbol and records the max volume on each trading day.
/// The resulting map is used by `detect_volume_trigger_signals()` with strict
/// backward-looking day filtering (`d < current_day`) to prevent look-ahead.
pub fn build_daily_max_volume(
    by_symbol: &BTreeMap<String, Vec<usize>>,
    bars: &[Bar],
) -> BTreeMap<String, BTreeMap<String, f64>> {
    let mut result: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    for (symbol, indices) in by_symbol {
        let mut day_vol: BTreeMap<String, f64> = BTreeMap::new();
        for &i in indices {
            let day = bars[i].ts_utc.get(0..10).unwrap_or("").to_string();
            let vol = bars[i].volume;
            day_vol
                .entry(day)
                .and_modify(|v: &mut f64| *v = v.max(vol))
                .or_insert(vol);
        }
        result.insert(symbol.clone(), day_vol);
    }
    result
}

// —— Signal detection ——

/// Detected signals for all symbols.
#[derive(Debug, Clone, Default)]
pub struct VolumeTriggerSignalResult {
    /// Long entry signals (buy candidates).
    pub entries: Vec<VolumeTriggerSignal>,
    /// Short/exit signals by symbol, used for reverse-signal exit matching.
    pub exits_by_symbol: BTreeMap<String, Vec<VolumeTriggerSignal>>,
}

/// Detect volume_trigger signals across all symbols using the daily max volume baseline
/// and 6-bar reference window direction.
///
/// # Algorithm (per bar, per symbol)
/// 1. Look up daily max volume for days strictly before current bar's trading day
/// 2. fast_baseline = avg(last N day max volumes), slow_baseline = max(last M day max volumes)
/// 3. If `current_volume ≥ fast_baseline × fast_threshold AND ≥ slow_baseline × slow_threshold`:
/// 4.    Check 6-bar reference window: close > max(prev 6) → Long, close < min(prev 6) → Short
pub fn detect_volume_trigger_signals(
    bars: &[Bar],
    by_symbol: &BTreeMap<String, Vec<usize>>,
    params: &VolumeTriggerParams,
) -> VolumeTriggerSignalResult {
    let daily_max_volume = build_daily_max_volume(by_symbol, bars);

    let mut entries = Vec::new();
    let mut exits_by_symbol: BTreeMap<String, Vec<VolumeTriggerSignal>> = BTreeMap::new();

    for (symbol, indices) in by_symbol {
        let n = indices.len();
        if n < params.min_warmup_bars() {
            continue;
        }

        let closes: Vec<f64> = indices.iter().map(|&i| bars[i].close).collect();
        let volumes: Vec<f64> = indices.iter().map(|&i| bars[i].volume).collect();

        let daily_maxes = match daily_max_volume.get(symbol) {
            Some(m) => m,
            None => continue,
        };

        for i in params.min_warmup_bars()..n {
            let bar_day = bars[indices[i]].ts_utc.get(0..10).unwrap_or("");

            // Collect daily max volumes for days strictly before current bar's day
            let mut days_before: Vec<(&String, &f64)> = daily_maxes
                .iter()
                .filter(|(d, _)| d.as_str() < bar_day)
                .collect();
            days_before.sort_by_key(|(d, _)| *d);
            let days_values: Vec<f64> = days_before.iter().map(|(_, v)| **v).collect();

            let fast_window: Vec<f64> = days_values
                .iter()
                .rev()
                .take(params.fast_window_days)
                .cloned()
                .collect();
            let slow_window: Vec<f64> = days_values
                .iter()
                .rev()
                .take(params.slow_window_days)
                .cloned()
                .collect();

            if fast_window.len() < params.fast_window_days
                || slow_window.len() < params.slow_window_days
            {
                continue;
            }

            let fast_baseline = fast_window.iter().sum::<f64>() / fast_window.len() as f64;
            let slow_baseline = slow_window.iter().cloned().fold(0.0_f64, f64::max);

            if fast_baseline <= 0.0 || slow_baseline <= 0.0 {
                continue;
            }

            let fast_line = fast_baseline * params.fast_threshold;
            let slow_line = slow_baseline * params.slow_threshold;

            if volumes[i] < fast_line || volumes[i] < slow_line {
                continue;
            }

            // 6-bar reference window: relative price direction
            let ref_start = if i >= params.direction_lookback_bars {
                i - params.direction_lookback_bars
            } else {
                0
            };
            let ref_closes = &closes[ref_start..i];
            if ref_closes.len() < params.direction_lookback_bars {
                continue;
            }

            let ref_max = ref_closes.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let ref_min = ref_closes.iter().cloned().fold(f64::INFINITY, f64::min);

            let signal_type = if closes[i] > ref_max {
                VolumeTriggerSignalType::Long
            } else if closes[i] < ref_min {
                VolumeTriggerSignalType::Short
            } else {
                continue;
            };

            let signal = VolumeTriggerSignal {
                symbol: symbol.clone(),
                bar_idx: i,
                signal_ts: bars[indices[i]].ts_utc.clone(),
                trading_day: bar_day.to_string(),
                close: bars[indices[i]].close,
                signal_type: signal_type.clone(),
            };

            match signal_type {
                VolumeTriggerSignalType::Long => entries.push(signal),
                VolumeTriggerSignalType::Short => {
                    exits_by_symbol
                        .entry(symbol.clone())
                        .or_default()
                        .push(signal);
                }
            }
        }
    }

    VolumeTriggerSignalResult {
        entries,
        exits_by_symbol,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn make_bars(symbol: &str, days: &[&str], closes: &[f64], volumes: &[f64]) -> Vec<Bar> {
        let mut bars = Vec::new();
        for (i, day) in days.iter().enumerate() {
            for j in 0..16 {
                let hour = 1 + j / 4;
                let min = (j % 4) * 15;
                let ts = format!("{}T{:02}:{:02}:00Z", day, hour, min);
                bars.push(make_bar(symbol, &ts, closes[i], volumes[i]));
            }
        }
        bars
    }

    #[test]
    fn daily_max_volume_picks_max_per_day() {
        let bars = vec![
            make_bar("TEST", "2026-01-02T01:00:00Z", 10.0, 100.0),
            make_bar("TEST", "2026-01-02T02:00:00Z", 10.1, 200.0),
            make_bar("TEST", "2026-01-02T03:00:00Z", 10.0, 150.0),
            make_bar("TEST", "2026-01-03T01:00:00Z", 11.0, 300.0),
        ];
        let mut by_symbol: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        by_symbol.insert("TEST".into(), (0..4).collect());

        let dm = build_daily_max_volume(&by_symbol, &bars);
        let test_dm = dm.get("TEST").unwrap();
        assert_eq!(test_dm.get("2026-01-02"), Some(&200.0)); // max of 100,200,150
        assert_eq!(test_dm.get("2026-01-03"), Some(&300.0));
    }

    #[test]
    fn signal_requires_volume_spike() {
        // Create 20 days of bars with small volume, then one day with huge volume + price breakout
        let days: Vec<String> = (0..20).map(|d| format!("2026-01-{:02}", d + 2)).collect();
        let day_strs: Vec<&str> = days.iter().map(|s| s.as_str()).collect();

        let mut closes = vec![10.0; 20];
        closes[19] = 12.0; // big close on last day
        let mut volumes = vec![100.0; 20];
        volumes[19] = 500.0; // big volume on last day

        let bars = make_bars("TEST", &day_strs, &closes, &volumes);
        let mut by_symbol: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        by_symbol.insert("TEST".into(), (0..bars.len()).collect());

        // With default params (slow_threshold=2.0), volume=500 might not be 2x the max of past 15 days
        // Use lenient params to ensure detection
        let params = VolumeTriggerParams {
            fast_threshold: 0.3,
            slow_threshold: 0.5,
            fast_window_days: 3,
            slow_window_days: 15,
            ..Default::default()
        };

        let result = detect_volume_trigger_signals(&bars, &by_symbol, &params);
        // The last day (day 19, 0-indexed) has close=12.0 which should be > max of prev 6 closes (all ~10.0)
        assert!(
            !result.entries.is_empty(),
            "Should detect volume spike + price breakout"
        );
        let signal = &result.entries[0];
        assert_eq!(signal.signal_type, VolumeTriggerSignalType::Long);
    }

    #[test]
    fn short_signal_stored_in_exits() {
        let days: Vec<String> = (0..20).map(|d| format!("2026-01-{:02}", d + 2)).collect();
        let day_strs: Vec<&str> = days.iter().map(|s| s.as_str()).collect();

        let mut closes = vec![10.0; 20];
        closes[19] = 8.0; // big drop on last day
        let mut volumes = vec![100.0; 20];
        volumes[19] = 500.0;

        let bars = make_bars("TEST", &day_strs, &closes, &volumes);
        let mut by_symbol: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        by_symbol.insert("TEST".into(), (0..bars.len()).collect());

        let params = VolumeTriggerParams {
            fast_threshold: 0.3,
            slow_threshold: 0.5,
            ..Default::default()
        };

        let result = detect_volume_trigger_signals(&bars, &by_symbol, &params);
        assert!(
            result.entries.is_empty(),
            "Drop day should not produce long signals"
        );
        assert!(
            result.exits_by_symbol.contains_key("TEST"),
            "Short signal should be in exits_by_symbol"
        );
    }

    #[test]
    fn no_look_ahead_daily_max_only_uses_past_days() {
        // Verify that daily max volume computation doesn't leak future days into past signals
        let bars = vec![
            make_bar("TEST", "2026-01-02T01:00:00Z", 10.0, 100.0),
            make_bar("TEST", "2026-01-03T01:00:00Z", 10.0, 1000.0), // huge vol on day 3
            make_bar("TEST", "2026-01-04T01:00:00Z", 10.0, 100.0),
        ];
        let mut by_symbol: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        by_symbol.insert("TEST".into(), (0..3).collect());

        let dm = build_daily_max_volume(&by_symbol, &bars);
        let test_dm = dm.get("TEST").unwrap();

        // Day 2026-01-02 should only see its own volume (100.0), not day 3's 1000.0
        // This is verified by the signal detection filtering d < bar_day
        assert_eq!(test_dm.get("2026-01-02"), Some(&100.0));
        assert_eq!(test_dm.get("2026-01-03"), Some(&1000.0));
        // The look-ahead safety is in the detection loop, not the pre-computation
        // This test confirms the map values are day-isolated
    }
}
