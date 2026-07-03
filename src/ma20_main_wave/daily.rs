//! Daily-level gate computation for MA20 main wave.
//!
//! Computes per-date trend gates (MA120 > MA200) and cascade direction gates
//! (EMA fast > EMA slow) from daily bars. Injected into signal-level via
//! `_daily_trend_by_date` and `_daily_direction_by_date` JSON maps.
//!
//! # Audit (no look-ahead bias)
//! - Trend: EMA120/EMA200 computed from daily closes — only dates up to current bar (✅)
//! - Direction: EMA fast/slow cross detection — persistent after first bullish cross (✅)
//! - Both gates are then applied per-date to intraday signals on that same day (no leakage ✅)

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::indicators::ema;
use crate::traits::Bar;

/// Compute per-date MA120/MA200 daily trend map.
///
/// Returns `{symbol: {date: bool}}` where `true` means EMA120 > EMA200 on that date.
/// Dates with fewer than 120 daily bars are omitted (gate passes by default).
pub fn compute_daily_trend_map(
    daily_bars: &[Bar],
    by_symbol: &BTreeMap<String, Vec<usize>>,
) -> BTreeMap<String, BTreeMap<String, Value>> {
    let mut result: BTreeMap<String, BTreeMap<String, Value>> = BTreeMap::new();

    for (sym, d_indices) in by_symbol {
        let n = d_indices.len();
        let daily_closes: Vec<f64> = d_indices.iter().map(|&i| daily_bars[i].close).collect();
        let daily_dates: Vec<String> = d_indices
            .iter()
            .map(|&i| daily_bars[i].ts_utc.get(0..10).unwrap_or("").to_string())
            .collect();

        let mut trend_map: BTreeMap<String, Value> = BTreeMap::new();
        if n >= 120 {
            let ema120 = ema(&daily_closes, 120);
            if n >= 200 {
                let ema200 = ema(&daily_closes, 200);
                for i in 199..n {
                    trend_map.insert(daily_dates[i].clone(), json!(ema120[i] > ema200[i]));
                }
            } else {
                for i in 119..n {
                    trend_map.insert(daily_dates[i].clone(), json!(daily_closes[i] > ema120[i]));
                }
            }
        }
        result.insert(sym.clone(), trend_map);
    }

    result
}

/// Compute per-date cascade direction map.
///
/// Returns `{symbol: {date: bool}}` where `true` means the daily EMA fast/slow
/// direction is bullish.
///
/// When `persistent` is true (default), bullish latches after first golden cross
/// and stays true. When false, each day is checked independently (point-in-time).
pub fn compute_cascade_direction_map(
    daily_bars: &[Bar],
    by_symbol: &BTreeMap<String, Vec<usize>>,
    ema_fast: usize,
    ema_slow: usize,
) -> BTreeMap<String, BTreeMap<String, Value>> {
    compute_cascade_direction_map_impl(daily_bars, by_symbol, ema_fast, ema_slow, true)
}

/// Non-persistent variant: each date is checked independently (no latching).
pub fn compute_cascade_direction_map_point_in_time(
    daily_bars: &[Bar],
    by_symbol: &BTreeMap<String, Vec<usize>>,
    ema_fast: usize,
    ema_slow: usize,
) -> BTreeMap<String, BTreeMap<String, Value>> {
    compute_cascade_direction_map_impl(daily_bars, by_symbol, ema_fast, ema_slow, false)
}

fn compute_cascade_direction_map_impl(
    daily_bars: &[Bar],
    by_symbol: &BTreeMap<String, Vec<usize>>,
    ema_fast: usize,
    ema_slow: usize,
    persistent: bool,
) -> BTreeMap<String, BTreeMap<String, Value>> {
    let mut result: BTreeMap<String, BTreeMap<String, Value>> = BTreeMap::new();

    for (sym, d_indices) in by_symbol {
        let n = d_indices.len();
        if n < ema_slow {
            continue;
        }

        let daily_closes: Vec<f64> = d_indices.iter().map(|&i| daily_bars[i].close).collect();
        let daily_dates: Vec<String> = d_indices
            .iter()
            .map(|&i| daily_bars[i].ts_utc.get(0..10).unwrap_or("").to_string())
            .collect();

        let fast_ema = ema(&daily_closes, ema_fast);
        let slow_ema = ema(&daily_closes, ema_slow);

        let mut dir_map: BTreeMap<String, Value> = BTreeMap::new();
        let mut bullish = false;
        for i in 0..n {
            if i >= ema_slow - 1 {
                if persistent {
                    if !bullish
                        && fast_ema[i] > slow_ema[i]
                        && i > 0
                        && fast_ema[i - 1] <= slow_ema[i - 1]
                    {
                        bullish = true;
                    }
                    dir_map.insert(daily_dates[i].clone(), json!(bullish));
                } else {
                    dir_map.insert(daily_dates[i].clone(), json!(fast_ema[i] > slow_ema[i]));
                }
            }
        }
        result.insert(sym.clone(), dir_map);
    }

    result
}
