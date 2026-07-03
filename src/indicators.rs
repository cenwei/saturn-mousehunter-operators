//! Pure math indicator functions — EMA, SMA, Bollinger Bands.
//!
//! Extracted from backtest-worker `indicators.rs` for shared use across operators.

use std::collections::VecDeque;

pub fn ema(values: &[f64], period: usize) -> Vec<f64> {
    if values.is_empty() {
        return Vec::new();
    }
    let period = period.max(1);
    let alpha = 2.0 / (period as f64 + 1.0);
    let mut out = Vec::with_capacity(values.len());
    out.push(values[0]);
    for value in values.iter().skip(1) {
        let next = value * alpha + out[out.len() - 1] * (1.0 - alpha);
        out.push(next);
    }
    out
}

pub fn sma(values: &[f64], period: usize) -> Vec<f64> {
    let period = period.max(1);
    let mut out = Vec::with_capacity(values.len());
    let mut window: VecDeque<f64> = VecDeque::new();
    let mut total = 0.0;
    for value in values {
        window.push_back(*value);
        total += *value;
        if window.len() > period {
            if let Some(first) = window.pop_front() {
                total -= first;
            }
        }
        out.push(total / window.len() as f64);
    }
    out
}

pub fn bollinger(values: &[f64], period: usize, std_multiplier: f64) -> Vec<(f64, f64, f64)> {
    let period = period.max(1);
    let mut out = Vec::with_capacity(values.len());
    let mut window: VecDeque<f64> = VecDeque::new();
    for value in values {
        window.push_back(*value);
        if window.len() > period {
            window.pop_front();
        }
        let mean = window.iter().copied().sum::<f64>() / window.len() as f64;
        let variance = window
            .iter()
            .map(|item| {
                let diff = item - mean;
                diff * diff
            })
            .sum::<f64>()
            / window.len() as f64;
        let stddev = variance.sqrt();
        out.push((
            mean,
            mean + std_multiplier * stddev,
            mean - std_multiplier * stddev,
        ));
    }
    out
}
