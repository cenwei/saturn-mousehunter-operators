//! Shared utilities for entry/exit enforcement.
//!
//! # Audit (no look-ahead bias)
//! - `enforce_t_plus_one`: only reads symbol name and config — static rule lookup (✅)
//! - `is_market_close_bar`: only reads UTC timestamp string — no future data (✅)
//! - `symbol_market`: static prefix-based market inference (✅)
//! - `bar_local_time_minutes`: only reads UTC timestamp — no future data (✅)
//! - `time_str_minutes`: pure string parsing (✅)

/// Infer market from symbol prefix (SZ/SH → CN, HK → HK, US → US).
pub fn symbol_market(symbol: &str) -> &str {
    if symbol.starts_with("HK") {
        "HK"
    } else if symbol.starts_with("US") {
        "US"
    } else {
        "CN"
    }
}

/// Parse ts_utc like "2026-06-01T06:30:00Z" into local-time minutes since midnight.
/// Market offset: CN/HK → +8h, US → -5h.
pub fn bar_local_time_minutes(ts_utc: &str, market: &str) -> Option<i32> {
    let hh: i32 = ts_utc.get(11..13)?.parse().ok()?;
    let mm: i32 = ts_utc.get(14..16)?.parse().ok()?;
    let offset: i32 = match market {
        "CN" | "HK" => 8,
        "US" => -5,
        _ => 0,
    };
    let total = hh * 60 + mm + offset * 60;
    Some(if total < 0 {
        total + 1440
    } else if total >= 1440 {
        total - 1440
    } else {
        total
    })
}

/// Parse "HH:MM" into total minutes.
pub fn time_str_minutes(s: &str) -> Option<i32> {
    let hh: i32 = s.get(0..2)?.parse().ok()?;
    let mm: i32 = s.get(3..5)?.parse().ok()?;
    Some(hh * 60 + mm)
}

/// Determine whether T+1 settlement applies.
///
/// Checks (in order):
/// 1. `t_plus_one` boolean parameter
/// 2. `settlement_rule` string ("t+1", "t1", "t_plus_one")
/// 3. `market` / `backtest_market` → CN market auto-enables T+1
/// 4. Symbol prefix heuristic (SH/SZ/*.SH/*.SZ) → CN stock
pub fn enforce_t_plus_one(parameters: &serde_json::Value, symbol: &str) -> bool {
    use serde_json::Value;

    if let Some(true) = parameters.get("t_plus_one").and_then(Value::as_bool) {
        return true;
    }
    if let Some(rule) = parameters.get("settlement_rule").and_then(Value::as_str) {
        return matches!(
            rule.to_ascii_lowercase().as_str(),
            "t+1" | "t1" | "t_plus_one"
        );
    }
    let market = parameters
        .get("market")
        .or_else(|| parameters.get("backtest_market"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if market.eq_ignore_ascii_case("CN") {
        return true;
    }
    if market.is_empty() {
        let s = symbol.trim().to_ascii_uppercase();
        return s.starts_with("SH") || s.starts_with("SZ") || s.ends_with(".SH") || s.ends_with(".SZ");
    }
    false
}

/// Check if a bar is a market close bar (last 15 minutes before close).
///
/// Aligned with Python `volume_trigger_detector.py` `_is_market_close_bar`.
/// Close bars use a higher fast threshold to filter auction noise.
pub fn is_market_close_bar(market: &str, ts_utc: &str) -> bool {
    if market.eq_ignore_ascii_case("CN") {
        if let Some(time_str) = ts_utc.get(11..16) {
            return time_str >= "06:45"; // CN close: >= 14:45 Beijing = 06:45 UTC
        }
    } else if market.eq_ignore_ascii_case("HK") {
        if let Some(time_str) = ts_utc.get(11..16) {
            return time_str >= "07:45"; // HK close: >= 15:45 HKT = 07:45 UTC
        }
    } else if market.eq_ignore_ascii_case("US") {
        if let Some(time_str) = ts_utc.get(11..16) {
            return time_str >= "19:45"; // US close: conservative (EST 20:45 / EDT 19:45 → use 19:45)
        }
    }
    false
}
