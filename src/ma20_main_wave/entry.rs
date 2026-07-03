//! Traditional entry signal detection for MA20 main wave.
//!
//! Three signal types, all backward-looking (no look-ahead):
//! - **boll_lower_rebound**: price touches then rebounds from Bollinger lower band
//! - **ema_pullback**: price pulls back to EMA20 in uptrend
//! - **strong_accumulation**: volume spike with price above Bollinger mid in uptrend
//!
//! # Audit
//! All conditions use only `i` and values computed from `[0..i]` windows — no future data.

/// Signal type detected per bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ma20SignalKind {
    BollLowerRebound,
    EmaPullback,
    StrongAccumulation,
    /// volume_trigger_short (exit signal, only produced in traditional path)
    VolumeTriggerShort,
}

impl Ma20SignalKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BollLowerRebound => "boll_lower_rebound",
            Self::EmaPullback => "ema_pullback",
            Self::StrongAccumulation => "strong_accumulation",
            Self::VolumeTriggerShort => "volume_trigger_short",
        }
    }
}

/// Result of evaluating traditional entry signal conditions for one bar.
pub struct Ma20SignalResult {
    pub signal_kind: Ma20SignalKind,
    pub strength: f64,
}

/// Evaluate traditional entry signals at bar index `i`.
///
/// All indicators (close, ema20, ma20_slope, vol_ratio, boll bands, avg_vol_20)
/// must be pre-computed for the full symbol and passed as slices.
pub fn detect_ma20_entry_signal(
    close: f64,
    open: f64,
    low: f64,
    boll_mid: f64,
    boll_upper: f64,
    boll_lower: f64,
    ema20: f64,
    ma20_slope: f64,
    vol_ratio: f64,
    volume: f64,
    avg_vol_20: f64,
) -> Option<Ma20SignalResult> {
    // boll_lower_rebound: low touches lower band, close rebounds above it (bullish candle)
    if low <= boll_lower * 1.02 && close > open && close > boll_lower {
        return Some(Ma20SignalResult {
            signal_kind: Ma20SignalKind::BollLowerRebound,
            strength: ((close - boll_lower) / boll_lower).min(0.05),
        });
    }

    // ema_pullback: close near EMA20 in uptrend, bullish candle, decent volume
    if close > ema20
        && close < ema20 * 1.03
        && ma20_slope > 0.0
        && close > open
        && vol_ratio > 0.8
    {
        return Some(Ma20SignalResult {
            signal_kind: Ma20SignalKind::EmaPullback,
            strength: (ma20_slope * 2.0).max(0.005).min(0.05),
        });
    }

    // strong_accumulation: high volume spike, price above mid band in uptrend
    if ma20_slope > 0.0005
        && vol_ratio > 1.5
        && close > boll_mid
        && close < boll_upper
        && volume > avg_vol_20 * 1.5
    {
        return Some(Ma20SignalResult {
            signal_kind: Ma20SignalKind::StrongAccumulation,
            strength: (vol_ratio / 3.0).min(0.05),
        });
    }

    None
}
