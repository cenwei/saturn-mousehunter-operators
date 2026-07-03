//! Exit computation for MA20 main wave positions.
//!
//! Implements three exit strategies (checked in order):
//! 1. **Main wave top exhaustion** — long upper shadow + volume spike above EMA20 in uptrend
//! 2. **Stop loss** — low reaches stop price
//! 3. **Profit take** — high reaches target price
//! 4. **Max holding** — fallback after max holding days exceeded
//!
//! # Audit (no look-ahead bias)
//! - Exit window: [entry_bar_idx + 1, entry_bar_idx + 1 + max_holding_days)
//! - All checks use bar data at index j within exit window (✅)
//! - EMA20 and SMA20 are computed from closes/volumes up to each check point (✅)

use crate::indicators::{ema, sma};
use crate::traits::Bar;

/// Exit information for a matched position.
#[derive(Debug, Clone)]
pub struct Ma20ExitInfo {
    pub exit_ts: String,
    pub exit_price: f64,
    pub exit_reason: String,
    pub holding_bars: usize,
}

/// Compute the exit for a MA20 entry position.
///
/// Parameters:
/// - `entry_bar_idx`: local bar index of entry (into per-symbol arrays)
/// - `entry_price`: fill price of the entry
/// - `closes`, `highs`, `lows`, `volumes`, `opens`: per-symbol arrays
/// - `indices`: per-symbol bar index → global bar index
/// - `bars`: global bars (used for timestamps and OHLCV lookup)
/// - `t_plus_one`: T+1 settlement — same-day exit is blocked
/// - `profit_take_threshold`, `stop_loss_pct`, `max_holding_trading_days`
/// - `mw_exit_enabled`, `mw_upper_shadow_ratio`, `mw_volume_spike_ratio`, `mw_require_bearish`
pub fn compute_exit(
    entry_bar_idx: usize,
    entry_price: f64,
    closes: &[f64],
    highs: &[f64],
    lows: &[f64],
    volumes: &[f64],
    opens: &[f64],
    indices: &[usize],
    bars: &[Bar],
    t_plus_one: bool,
    profit_take_threshold: f64,
    stop_loss_pct: f64,
    max_holding_trading_days: usize,
    mw_exit_enabled: bool,
    mw_upper_shadow_ratio: f64,
    mw_volume_spike_ratio: f64,
    mw_require_bearish: bool,
) -> Option<Ma20ExitInfo> {
    let n = closes.len();
    let target_price = entry_price * (1.0 + profit_take_threshold);
    let stop_price = entry_price * (1.0 - stop_loss_pct);

    // Determine exit window end: advance up to max_holding_trading_days
    let mut exit_window_end = entry_bar_idx + 1;
    let mut seen_days = std::collections::BTreeSet::new();
    for j in (entry_bar_idx + 1)..n {
        if let Some(day) = bars[indices[j]].ts_utc.get(0..10) {
            seen_days.insert(day.to_string());
            if seen_days.len() > max_holding_trading_days {
                break;
            }
        }
        exit_window_end = j + 1;
    }
    if exit_window_end <= entry_bar_idx + 1 {
        return None;
    }

    // T+1: skip same-day bars
    let mut exit_start = entry_bar_idx + 1;
    if t_plus_one {
        let entry_date = &bars[indices[entry_bar_idx]].ts_utc[..10];
        while exit_start < exit_window_end
            && bars[indices[exit_start]].ts_utc.starts_with(entry_date)
        {
            exit_start += 1;
        }
    }
    if exit_start >= exit_window_end {
        return None;
    }

    let ema20_vals = ema(closes, 20);
    let sma20_vol = sma(volumes, 20);

    let mut exit_idx = exit_window_end - 1;
    let mut exit_reason = "max_holding";
    let mut exit_price = closes[exit_idx];

    for j in exit_start..exit_window_end {
        // Main-wave exit: top exhaustion pattern
        if mw_exit_enabled && j >= 20 {
            let ma20_slope = if j >= 5 && ema20_vals[j - 5] > 0.0 {
                (ema20_vals[j] - ema20_vals[j - 5]) / ema20_vals[j - 5]
            } else {
                0.0
            };
            if closes[j] > ema20_vals[j] && ma20_slope > 0.0 {
                let body_top = opens[j].max(closes[j]);
                let candle_range = highs[j] - lows[j];
                if candle_range > 0.0 {
                    let upper_shadow = highs[j] - body_top;
                    let shadow_ratio = upper_shadow / candle_range;
                    let vol_ratio = if sma20_vol[j] > 0.0 {
                        volumes[j] / sma20_vol[j]
                    } else {
                        0.0
                    };
                    if shadow_ratio >= mw_upper_shadow_ratio
                        && vol_ratio >= mw_volume_spike_ratio
                        && (!mw_require_bearish || closes[j] < opens[j])
                    {
                        exit_idx = j;
                        exit_price = closes[j];
                        exit_reason = "main_wave_top_exhaustion";
                        break;
                    }
                }
            }
        }

        // Stop loss
        if lows[j] <= stop_price {
            exit_idx = j;
            exit_price = if bars[indices[j]].open < stop_price {
                bars[indices[j]].open
            } else {
                stop_price
            };
            exit_reason = "stop_loss";
            break;
        }
        // Profit take
        if highs[j] >= target_price {
            exit_idx = j;
            exit_price = target_price;
            exit_reason = "profit_take";
            break;
        }
    }

    Some(Ma20ExitInfo {
        exit_ts: bars[indices[exit_idx]].ts_utc.clone(),
        exit_price,
        exit_reason: exit_reason.to_string(),
        holding_bars: exit_idx - entry_bar_idx,
    })
}
