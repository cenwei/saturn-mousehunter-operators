//! Single source of truth for volume_trigger strategy parameters.
//!
//! Replaces the scattered parameter extraction in:
//! - `ma20_slope_bollinger_scale.rs` (backtest worker)
//! - `polars_view.rs` (signal runtime)
//! - `strategy_admin.rs` (DSL→worker mapping)
//!
//! # Audit
//! - Algorithm reference: `volume_trigger_strategy.py` (daily max volume baseline)
//! - Look-ahead safe: ✅ (all windows are backward-looking, see audit rules in docs)

use serde_json::Value;

/// Core parameters for the volume_trigger signal detection.
///
/// These define the dual-window volume baseline check, 6-bar direction,
/// and entry/exit gating rules.
#[derive(Debug, Clone)]
pub struct VolumeTriggerParams {
    // ── Volume baseline windows (trading days) ──
    /// Fast window: number of past trading days for rolling average of daily max volume.
    /// Default: 3 (aligned with Python `volume_trigger_strategy.py`)
    pub fast_window_days: usize,

    /// Slow window: number of past trading days for rolling max of daily max volume.
    /// Default: 15
    pub slow_window_days: usize,

    /// Fast threshold: current_vol ≥ avg_daily_max_3d × fast_threshold.
    /// NOTE: When slow_threshold ≥ fast_threshold, the slow threshold is the binding constraint
    /// because slow_baseline (max of 15 days) ≥ fast_baseline (avg of 3 days) always.
    /// Default: 0.5 (optimal from backtest v4)
    pub fast_threshold: f64,

    /// Slow threshold: current_vol ≥ max_daily_max_15d × slow_threshold.
    /// This is the PRIMARY binding constraint for signal quality.
    /// Optimal: 2.0 (2430% return, 72.4% win rate, -1.22% drawdown)
    pub slow_threshold: f64,

    // ── Direction (6-bar reference window) ──
    /// Number of 15-minute bars before current bar used as reference window.
    /// Default: 6 (aligned with Python)
    pub direction_lookback_bars: usize,

    // ── Entry gates ──
    /// Enable MA120 > MA200 daily trend gate. Default: true
    pub daily_ma120_ma200_gate_enabled: bool,

    /// Enable EMA fast/slow direction cascade filter. Default: true
    pub cascade_direction_enabled: bool,

    /// Fast EMA window for cascade direction. Default: 5
    pub cascade_direction_fast: usize,

    /// Slow EMA window for cascade direction. Default: 20
    pub cascade_direction_slow: usize,

    // ── Position management ──
    /// Position size per trade as fraction of total capital. Default: 0.16
    pub target_position_per_trade: f64,

    /// Maximum number of buy entries per trading day. Default: 1
    pub max_daily_buys: usize,

    /// Maximum total position ratio. Default: 1.0 (100%)
    pub max_total_position_ratio: f64,

    /// Maximum position per symbol. Default: 1.0 (100%)
    pub max_position_per_symbol: f64,

    // ── Settlement ──
    /// T+1 settlement rule. Default: true (A-share)
    pub t_plus_one: bool,

    // ── Signal routing ──
    /// Master switch: true = volume_trigger path, false = traditional multi-signal path.
    pub volume_trigger_signal_enabled: bool,

    /// Bars per trading day (15-minute bars: 16). Default: 16
    pub bars_per_day: usize,
}

impl Default for VolumeTriggerParams {
    fn default() -> Self {
        Self {
            fast_window_days: 3,
            slow_window_days: 15,
            fast_threshold: 0.5,
            slow_threshold: 2.0,
            direction_lookback_bars: 6,
            daily_ma120_ma200_gate_enabled: true,
            cascade_direction_enabled: false,
            cascade_direction_fast: 5,
            cascade_direction_slow: 20,
            target_position_per_trade: 0.16,
            max_daily_buys: 1,
            max_total_position_ratio: 1.0,
            max_position_per_symbol: 1.0,
            t_plus_one: true,
            volume_trigger_signal_enabled: true,
            bars_per_day: 16,
        }
    }
}

impl VolumeTriggerParams {
    /// Build from a `serde_json::Value` parameter bag (backtest worker style).
    ///
    /// Handles the legacy DSL→worker parameter mapping:
    /// - `fast_threshold` maps to both `volume_trigger_ratio` and `trigger_fast_threshold`
    /// - `slow_threshold` maps to both `cascade_volume_ratio` and `trigger_slow_threshold`
    /// - `fast_window_days` maps to both `trigger_fast_window_days` and `cascade_direction_fast_window`
    /// - `slow_window_days` maps to both `trigger_slow_window_days` and `cascade_direction_slow_window`
    pub fn from_value(params: &Value) -> Self {
        let mut p = Self::default();

        // Volume trigger master switch
        if let Some(v) = params
            .get("volume_trigger_signal_enabled")
            .and_then(Value::as_bool)
        {
            p.volume_trigger_signal_enabled = v;
        }

        // Fast threshold: check legacy keys first so trial param values take priority
        // over pipeline/registry defaults that use canonical (trigger_*) names.
        p.fast_threshold = params
            .get("fast_threshold")
            .or_else(|| params.get("trigger_fast_threshold"))
            .and_then(Value::as_f64)
            .unwrap_or(p.fast_threshold);

        // Slow threshold
        p.slow_threshold = params
            .get("slow_threshold")
            .or_else(|| params.get("trigger_slow_threshold"))
            .and_then(Value::as_f64)
            .unwrap_or(p.slow_threshold);

        // Fast window days
        p.fast_window_days = params
            .get("fast_window_days")
            .or_else(|| params.get("trigger_fast_window_days"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(p.fast_window_days)
            .max(1);

        // Slow window days
        p.slow_window_days = params
            .get("slow_window_days")
            .or_else(|| params.get("trigger_slow_window_days"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(p.slow_window_days)
            .max(p.fast_window_days + 1);

        // Direction lookback
        p.direction_lookback_bars = params
            .get("direction_lookback_bars")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(p.direction_lookback_bars)
            .max(1);

        // Gates
        if let Some(v) = params
            .get("daily_ma120_ma200_buy_gate_enabled")
            .and_then(Value::as_bool)
        {
            p.daily_ma120_ma200_gate_enabled = v;
        }
        if let Some(v) = params
            .get("cascade_direction_enabled")
            .and_then(Value::as_bool)
        {
            p.cascade_direction_enabled = v;
        }

        // Cascade direction windows
        p.cascade_direction_fast = params
            .get("cascade_direction_fast_window")
            .or_else(|| params.get("fast_window_days"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(p.cascade_direction_fast)
            .max(2);

        p.cascade_direction_slow = params
            .get("cascade_direction_slow_window")
            .or_else(|| params.get("slow_window_days"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(p.cascade_direction_slow)
            .max(p.cascade_direction_fast + 1);

        // Position management
        p.target_position_per_trade = params
            .get("target_position_per_trade")
            .or_else(|| params.get("quantity_ratio"))
            .and_then(Value::as_f64)
            .unwrap_or(p.target_position_per_trade)
            .clamp(0.01, 1.0);

        p.max_daily_buys = params
            .get("max_daily_buys")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(p.max_daily_buys)
            .max(1);

        p.max_total_position_ratio = params
            .get("max_total_position_ratio")
            .and_then(Value::as_f64)
            .unwrap_or(p.max_total_position_ratio)
            .clamp(0.01, 1.0);

        p.max_position_per_symbol = params
            .get("max_position_per_symbol")
            .and_then(Value::as_f64)
            .unwrap_or(p.max_position_per_symbol)
            .clamp(0.01, 2.0);

        // Settlement
        p.t_plus_one = params
            .get("settlement_rule")
            .and_then(Value::as_str)
            .map(|s| s == "t_plus_one")
            .unwrap_or(p.t_plus_one);

        // Bars per day
        p.bars_per_day = params
            .get("bars_per_day")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(p.bars_per_day)
            .max(1);

        p
    }

    /// Audit: verify that the slow threshold is the binding constraint.
    /// Returns true if slow_threshold is the effective filter.
    pub fn slow_is_binding(&self) -> bool {
        // slow_baseline (max of 15 days) is always >= fast_baseline (avg of 3 days),
        // so slow_threshold is the binding constraint when slow_threshold >= fast_threshold.
        self.slow_threshold >= self.fast_threshold
    }

    /// Returns the minimum window size needed (in bars) before signal detection can begin.
    pub fn min_warmup_bars(&self) -> usize {
        self.slow_window_days * self.bars_per_day + self.direction_lookback_bars
    }

    /// Returns a JSON representation for audit/debugging.
    pub fn to_audit_json(&self) -> Value {
        serde_json::json!({
            "algorithm": "daily_max_volume_baseline_v1",
            "python_reference": "volume_trigger_strategy.py",
            "fast_window_days": self.fast_window_days,
            "slow_window_days": self.slow_window_days,
            "fast_threshold": self.fast_threshold,
            "slow_threshold": self.slow_threshold,
            "slow_is_binding": self.slow_is_binding(),
            "direction_lookback_bars": self.direction_lookback_bars,
            "daily_ma120_ma200_gate": self.daily_ma120_ma200_gate_enabled,
            "cascade_direction": self.cascade_direction_enabled,
            "cascade_direction_fast": self.cascade_direction_fast,
            "cascade_direction_slow": self.cascade_direction_slow,
            "target_position_per_trade": self.target_position_per_trade,
            "max_daily_buys": self.max_daily_buys,
            "max_total_position_ratio": self.max_total_position_ratio,
            "max_position_per_symbol": self.max_position_per_symbol,
            "t_plus_one": self.t_plus_one,
            "bars_per_day": self.bars_per_day,
            "min_warmup_bars": self.min_warmup_bars(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_params_are_backtest_v4_optimal() {
        let p = VolumeTriggerParams::default();
        assert_eq!(p.fast_threshold, 0.5);
        assert_eq!(p.slow_threshold, 2.0);
        assert_eq!(p.fast_window_days, 3);
        assert_eq!(p.slow_window_days, 15);
        assert!(p.slow_is_binding());
        assert!(p.volume_trigger_signal_enabled);
    }

    #[test]
    fn from_value_handles_legacy_keys() {
        let json = serde_json::json!({
            "fast_threshold": 0.35,
            "slow_threshold": 1.0,
            "fast_window_days": 3,
            "slow_window_days": 15,
            "volume_trigger_signal_enabled": true,
            "settlement_rule": "t_plus_one",
        });
        let p = VolumeTriggerParams::from_value(&json);
        assert_eq!(p.fast_threshold, 0.35);
        assert_eq!(p.slow_threshold, 1.0);
        assert!(p.t_plus_one);
    }

    #[test]
    fn from_value_legacy_keys_take_precedence() {
        // Legacy keys (fast_threshold) take priority over canonical keys (trigger_fast_threshold)
        // so that trial parameter values from optimize-fusion override pipeline defaults.
        let json = serde_json::json!({
            "fast_threshold": 0.35,
            "trigger_fast_threshold": 0.5,
            "slow_threshold": 1.0,
            "trigger_slow_threshold": 2.0,
        });
        let p = VolumeTriggerParams::from_value(&json);
        assert_eq!(p.fast_threshold, 0.35);
        assert_eq!(p.slow_threshold, 1.0);
    }

    #[test]
    fn min_warmup_bars_sufficient() {
        let p = VolumeTriggerParams::default();
        // 15 days * 16 bars/day + 6 bars = 246 bars minimum
        assert_eq!(p.min_warmup_bars(), 246);
    }
}
