// saturn-mousehunter-operators — shared operator library
//
// Single source of truth for all strategy operator logic.
// Used by both backtest-rust-worker and signal-runtime.

pub mod indicators;
pub mod ma20_main_wave;
pub mod registry;
pub mod traits;
pub mod volume_trigger;
