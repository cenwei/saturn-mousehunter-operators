//! Static operator registry — single source of truth for operator metadata.
//!
//! Used by backtest-worker (capability listing, plan validation, alias resolution)
//! and signal-runtime (AI DSL generation, operator classification).
//!
//! Each operator implementation file still owns its `OperatorMeta`, but the
//! canonical list of known operators lives here so both consumers stay in sync.

use std::sync::LazyLock;

use serde_json::{Value, json};

/// Minimal operator metadata entry in the shared registry.
#[derive(Debug, Clone)]
pub struct OperatorEntry {
    pub operator_key: &'static str,
    pub operator_type: &'static str,
    pub execution_model: &'static str, // "columnar" or "bars"
    pub aliases: &'static [&'static str],
    pub dependencies: &'static [&'static str],
    pub output_columns: &'static [&'static str],
    pub input_fields_zh: Value,
    pub output_fields_zh: Value,
}

impl OperatorEntry {
    pub fn to_capability(
        &self,
        contract_version: &str,
        volume_unit: &str,
        required_columns: &[&str],
        optional_columns: &[&str],
    ) -> Value {
        json!({
            "operator_key": self.operator_key,
            "operator_type": self.operator_type,
            "runtime": "rust",
            "status": "stable",
            "input_contract": {
                "contract_version": contract_version,
                "execution_model": self.execution_model,
                "volume_unit": volume_unit,
                "legacy_row_adapter_used": false,
                "depends_on": self.dependencies,
                "required_columns": required_columns,
                "optional_columns": optional_columns,
            },
            "output_columns": self.output_columns,
            "input_fields_zh": self.input_fields_zh,
            "output_schema": {
                "fields_zh": self.output_fields_zh
            }
        })
    }
}

macro_rules! zh {
    ($($s:expr),* $(,)?) => {
        json!([$($s),*])
    };
}

pub static ALL_OPERATORS: LazyLock<Vec<OperatorEntry>> = LazyLock::new(|| {
    vec![
        // —— Columnar operators ——
        OperatorEntry {
            operator_key: "atr_breakout_v1",
            operator_type: "indicator",
            execution_model: "columnar",
            aliases: &["atr_breakout_v1"],
            dependencies: &[],
            output_columns: &["atr_value", "atr_upper", "atr_lower", "breakout_signal"],
            input_fields_zh: zh!["价格", "成交量"],
            output_fields_zh: zh!["atr_value", "atr_upper", "atr_lower", "breakout_signal"],
        },
        OperatorEntry {
            operator_key: "bollinger_bands_v1",
            operator_type: "indicator",
            execution_model: "columnar",
            aliases: &["bollinger_bands"],
            dependencies: &[],
            output_columns: &["boll_mid", "boll_upper", "boll_lower", "boll_width", "boll_phase"],
            input_fields_zh: zh!["价格"],
            output_fields_zh: zh!["布林中轨", "布林上轨", "布林下轨", "布林带宽", "布林阶段"],
        },
        OperatorEntry {
            operator_key: "boll_upper_top_v1",
            operator_type: "exit",
            execution_model: "columnar",
            aliases: &["rust_builtin.boll_upper_top_v1"],
            dependencies: &["bollinger_bands_v1"],
            output_columns: &["boll_upper_top_signal"],
            input_fields_zh: zh!["K线数据", "布林带"],
            output_fields_zh: zh!["布林上轨出场信号"],
        },
        OperatorEntry {
            operator_key: "daily_score_weighted_sizing_v1",
            operator_type: "portfolio",
            execution_model: "columnar",
            aliases: &["rust_builtin.daily_score_weighted_sizing_v1"],
            dependencies: &[],
            output_columns: &["position_allocation"],
            input_fields_zh: zh!["入场信号", "组合参数"],
            output_fields_zh: zh!["仓位分配"],
        },
        OperatorEntry {
            operator_key: "ema5_ema10_adhesion_signal",
            operator_type: "indicator",
            execution_model: "columnar",
            aliases: &["ema5_ema10_adhesion_signal"],
            dependencies: &[],
            output_columns: &[],
            input_fields_zh: zh!["价格", "成交量"],
            output_fields_zh: json!([]),
        },
        OperatorEntry {
            operator_key: "ema_slope_v1",
            operator_type: "indicator",
            execution_model: "columnar",
            aliases: &["ema_slope"],
            dependencies: &[],
            output_columns: &[],
            input_fields_zh: zh!["价格"],
            output_fields_zh: zh!["EMA5", "EMA10", "EMA20", "SMA20", "MA20斜率", "MA20斜率比", "EMA5-EMA20差值"],
        },
        OperatorEntry {
            operator_key: "end_of_backtest_liquidation_v1",
            operator_type: "exit",
            execution_model: "columnar",
            aliases: &["rust_builtin.end_of_backtest_liquidation_v1"],
            dependencies: &[],
            output_columns: &["liquidation_signal"],
            input_fields_zh: zh!["K线数据"],
            output_fields_zh: zh!["回测结束清仓信号"],
        },
        OperatorEntry {
            operator_key: "history_signal_volume_entry_v1",
            operator_type: "entry",
            execution_model: "columnar",
            aliases: &["rust_builtin.history_signal_volume_entry_v1"],
            dependencies: &[],
            output_columns: &["volume_entry_signal"],
            input_fields_zh: zh!["K线数据"],
            output_fields_zh: zh!["历史量能入场信号"],
        },
        OperatorEntry {
            operator_key: "history_signal_volume_gate_v1",
            operator_type: "signal",
            execution_model: "columnar",
            aliases: &["rust_builtin.history_signal_volume_gate_v1"],
            dependencies: &[],
            output_columns: &["volume_gate"],
            input_fields_zh: zh!["K线数据", "历史信号"],
            output_fields_zh: zh!["历史量能门控信号"],
        },
        OperatorEntry {
            operator_key: "kline_dataset_v1",
            operator_type: "input",
            execution_model: "columnar",
            aliases: &["rust_builtin.kline_dataset_v1"],
            dependencies: &[],
            output_columns: &["ts_utc", "open", "high", "low", "close", "volume"],
            input_fields_zh: json!([]),
            output_fields_zh: zh!["K线数据帧"],
        },
        OperatorEntry {
            operator_key: "low_buy_recovery_v1",
            operator_type: "entry",
            execution_model: "columnar",
            aliases: &["rust_builtin.low_buy_recovery_v1"],
            dependencies: &["bollinger_bands_v1", "ema_slope_v1"],
            output_columns: &["low_buy_recovery_signal"],
            input_fields_zh: zh!["K线数据", "布林带", "EMA斜率"],
            output_fields_zh: zh!["低位回补入场信号"],
        },
        OperatorEntry {
            operator_key: "ma20_entry_signal_v1",
            operator_type: "signal",
            execution_model: "columnar",
            aliases: &["ma20_entry_signal"],
            dependencies: &["bollinger_bands_v1", "ema_slope_v1", "volume_profile_v1"],
            output_columns: &[],
            input_fields_zh: zh!["价格", "成交量", "布林带", "EMA斜率", "成交量概况"],
            output_fields_zh: zh!["入场信号", "入场强度", "入场价格"],
        },
        OperatorEntry {
            operator_key: "ma20_exit_signal_v1",
            operator_type: "signal",
            execution_model: "columnar",
            aliases: &["ma20_exit_signal"],
            dependencies: &["bollinger_bands_v1", "ema_slope_v1"],
            output_columns: &[],
            input_fields_zh: zh!["价格", "布林带", "EMA斜率"],
            output_fields_zh: zh!["出场信号", "出场原因"],
        },
        OperatorEntry {
            operator_key: "ma20_trend_break_v1",
            operator_type: "exit",
            execution_model: "columnar",
            aliases: &["rust_builtin.ma20_trend_break_v1"],
            dependencies: &["ema_slope_v1"],
            output_columns: &["trend_break_signal"],
            input_fields_zh: zh!["K线数据", "EMA斜率"],
            output_fields_zh: zh!["趋势破坏出场信号"],
        },
        OperatorEntry {
            operator_key: "ma_cross_v1",
            operator_type: "signal",
            execution_model: "columnar",
            aliases: &["ma_cross"],
            dependencies: &[],
            output_columns: &["cross_signal", "cross_strength", "fast_ma", "slow_ma"],
            input_fields_zh: zh!["价格"],
            output_fields_zh: zh!["信号"],
        },
        OperatorEntry {
            operator_key: "main_wave_exit_v1",
            operator_type: "exit",
            execution_model: "columnar",
            aliases: &["main_wave_exit"],
            dependencies: &[],
            output_columns: &["exit_signal", "signals"],
            input_fields_zh: zh!["K线数据"],
            output_fields_zh: zh!["主升浪见顶退出信号"],
        },
        OperatorEntry {
            operator_key: "max_holding_v1",
            operator_type: "exit",
            execution_model: "columnar",
            aliases: &["rust_builtin.max_holding_v1"],
            dependencies: &[],
            output_columns: &["max_holding_signal"],
            input_fields_zh: zh!["K线数据"],
            output_fields_zh: zh!["最大持仓天数信号"],
        },
        OperatorEntry {
            operator_key: "position_manager_v1",
            operator_type: "execution",
            execution_model: "columnar",
            aliases: &["position_manager"],
            dependencies: &[],
            output_columns: &[],
            input_fields_zh: zh!["交易信号列表"],
            output_fields_zh: zh!["订单", "成交", "回测报告", "持仓摘要", "市场风险状态"],
        },
        OperatorEntry {
            operator_key: "profit_take_v1",
            operator_type: "exit",
            execution_model: "columnar",
            aliases: &["rust_builtin.profit_take_v1"],
            dependencies: &[],
            output_columns: &["take_profit_signal"],
            input_fields_zh: zh!["K线数据"],
            output_fields_zh: zh!["止盈信号"],
        },
        OperatorEntry {
            operator_key: "red_green_ratio_16_v1",
            operator_type: "indicator",
            execution_model: "columnar",
            aliases: &["red_green_ratio_16"],
            dependencies: &[],
            output_columns: &["red_ratio_16", "green_ratio_16"],
            input_fields_zh: zh!["K线数据"],
            output_fields_zh: zh!["16周期绿盘占比", "16周期红盘占比"],
        },
        OperatorEntry {
            operator_key: "red_green_ratio_signal_v1",
            operator_type: "signal",
            execution_model: "columnar",
            aliases: &["red_green_ratio_signal"],
            dependencies: &["red_green_ratio_16_v1"],
            output_columns: &[],
            input_fields_zh: zh!["K线数据", "红绿比率"],
            output_fields_zh: zh!["多头信号", "空头信号"],
        },
        OperatorEntry {
            operator_key: "rsi_reversal_v1",
            operator_type: "signal",
            execution_model: "columnar",
            aliases: &["rsi_14"],
            dependencies: &[],
            output_columns: &[],
            input_fields_zh: zh!["价格"],
            output_fields_zh: zh!["信号"],
        },
        OperatorEntry {
            operator_key: "strong_accumulation_context_v1",
            operator_type: "entry",
            execution_model: "columnar",
            aliases: &["rust_builtin.strong_accumulation_context_v1"],
            dependencies: &["bollinger_bands_v1", "ema_slope_v1"],
            output_columns: &["accumulation_signal"],
            input_fields_zh: zh!["K线数据", "布林带", "EMA斜率"],
            output_fields_zh: zh!["强吸筹入场信号"],
        },
        OperatorEntry {
            operator_key: "volume_profile_v1",
            operator_type: "indicator",
            execution_model: "columnar",
            aliases: &["volume_profile"],
            dependencies: &[],
            output_columns: &["avg_volume_20", "volume_ratio_20"],
            input_fields_zh: zh!["成交量"],
            output_fields_zh: zh!["20周期均量", "20周期量比"],
        },
        OperatorEntry {
            operator_key: "volume_trigger_v1",
            operator_type: "indicator",
            execution_model: "columnar",
            aliases: &["volume_trigger"],
            dependencies: &[],
            output_columns: &["volume_ratio", "volume_trigger"],
            input_fields_zh: zh!["成交量"],
            output_fields_zh: zh!["量比", "量触发信号"],
        },
        OperatorEntry {
            operator_key: "window_kline_updown_stats_v1",
            operator_type: "indicator",
            execution_model: "columnar",
            aliases: &["window_kline_updown_stats"],
            dependencies: &[],
            output_columns: &["up_count", "down_count", "first_close", "last_close"],
            input_fields_zh: zh!["K线数据"],
            output_fields_zh: zh!["快窗口上涨根数", "慢窗口下跌根数", "快窗口首根收盘价", "快窗口末根收盘价"],
        },
        OperatorEntry {
            operator_key: "window_up_buy_signal_v1",
            operator_type: "signal",
            execution_model: "columnar",
            aliases: &["window_up_buy_signal"],
            dependencies: &["window_kline_updown_stats_v1"],
            output_columns: &["buy_signal", "up_down_ratio", "price_change_pct"],
            input_fields_zh: zh!["K线数据", "窗口涨跌统计(up_count/down_count/first_close/last_close)"],
            output_fields_zh: zh!["买入信号(0/1)", "涨跌比(up/down)", "窗口涨幅"],
        },
        OperatorEntry {
            operator_key: "ma20_composite_v1",
            operator_type: "strategy",
            execution_model: "columnar",
            aliases: &["ma20_composite"],
            dependencies: &["ma20_entry_signal_v1", "ma20_exit_signal_v1", "position_manager_v1"],
            output_columns: &[],
            input_fields_zh: zh!["价格", "成交量"],
            output_fields_zh: zh!["交易信号", "订单", "成交", "回测报告"],
        },
        // —— Bars operators ——
        OperatorEntry {
            operator_key: "cascade_gate_v1",
            operator_type: "gate",
            execution_model: "bars",
            aliases: &["cascade_gate"],
            dependencies: &[],
            output_columns: &["cascade_signal", "signals"],
            input_fields_zh: zh!["K线数据", "日线数据"],
            output_fields_zh: zh!["级联信号", "合并信号"],
        },
        OperatorEntry {
            operator_key: "ema_cluster_breakout_v1",
            operator_type: "strategy",
            execution_model: "bars",
            aliases: &["ema_cluster_breakout"],
            dependencies: &[],
            output_columns: &[],
            input_fields_zh: json!([]),
            output_fields_zh: json!([]),
        },
        OperatorEntry {
            operator_key: "main_wave_continuation_v1",
            operator_type: "entry",
            execution_model: "bars",
            aliases: &["rust_builtin.main_wave_continuation_v1"],
            dependencies: &["ema_cluster_breakout_v1"],
            output_columns: &["main_wave_continuation_signal"],
            input_fields_zh: zh!["K线数据", "日K线", "EMA突破信号"],
            output_fields_zh: zh!["主升浪延续确认信号"],
        },
        OperatorEntry {
            operator_key: "main_wave_verifier_v1",
            operator_type: "indicator",
            execution_model: "bars",
            aliases: &["main_wave_verifier"],
            dependencies: &[],
            output_columns: &[],
            input_fields_zh: json!([]),
            output_fields_zh: json!([]),
        },
        OperatorEntry {
            operator_key: "ma20_slope_bollinger_scale_v1",
            operator_type: "strategy",
            execution_model: "bars",
            aliases: &["ma20_slope_bollinger_scale"],
            dependencies: &[],
            output_columns: &[],
            input_fields_zh: zh!["价格", "成交量", "时间"],
            output_fields_zh: zh!["交易信号", "订单", "成交记录", "回测报告", "策略审计", "持仓摘要"],
        },
        OperatorEntry {
            operator_key: "tail_low_rebound_boll_shadow_v1",
            operator_type: "strategy",
            execution_model: "bars",
            aliases: &["tail_low_rebound_boll_shadow"],
            dependencies: &[],
            output_columns: &["entry_trigger", "exit_trigger", "position_pct"],
            input_fields_zh: zh!["价格", "成交量", "时间", "日K线"],
            output_fields_zh: zh!["入场信号", "出场信号", "持仓比例"],
        },
    ]
});

/// Resolve a raw operator key or alias to its canonical key.
pub fn resolve_alias(raw: &str) -> Option<&'static str> {
    let trimmed = raw.trim();
    for entry in ALL_OPERATORS.iter() {
        if entry.operator_key == trimmed {
            return Some(entry.operator_key);
        }
        for alias in entry.aliases {
            if *alias == trimmed {
                return Some(entry.operator_key);
            }
        }
    }
    None
}

/// Legacy plan name → operator key inference.
pub fn infer_from_plan_name(plan_name: &str) -> Option<&'static str> {
    if plan_name.contains("tail_low_rebound_boll_shadow") {
        Some("tail_low_rebound_boll_shadow_v1")
    } else if plan_name.contains("volume_trigger") {
        Some("volume_trigger_v1")
    } else if plan_name.contains("ema_cluster_breakout") {
        Some("ema_cluster_breakout_v1")
    } else if plan_name.contains("main_wave_verifier") {
        Some("main_wave_verifier_v1")
    } else if plan_name.contains("bollinger_bands") {
        Some("bollinger_bands_v1")
    } else if plan_name.contains("volume_profile") {
        Some("volume_profile_v1")
    } else if plan_name.contains("ema_slope") {
        Some("ema_slope_v1")
    } else if plan_name.contains("ma20_entry_signal") {
        Some("ma20_entry_signal_v1")
    } else if plan_name.contains("ma20_exit_signal") {
        Some("ma20_exit_signal_v1")
    } else if plan_name.contains("position_manager") {
        Some("position_manager_v1")
    } else if plan_name.contains("ma20_composite") {
        Some("ma20_composite_v1")
    } else if plan_name.contains("ma_cross") {
        Some("ma_cross_v1")
    } else if plan_name.contains("ema5_ema10_adhesion") {
        Some("ema5_ema10_adhesion_signal")
    } else if plan_name.contains("atr_breakout") {
        Some("atr_breakout_v1")
    } else if plan_name.contains("window_kline_updown_stats") {
        Some("window_kline_updown_stats_v1")
    } else if plan_name.contains("window_up_buy_signal") {
        Some("window_up_buy_signal_v1")
    } else if plan_name.contains("rsi") {
        Some("rsi_reversal_v1")
    } else if plan_name.contains("ma20") {
        Some("ma20_slope_bollinger_scale_v1")
    } else {
        None
    }
}

/// Check if a compiled plan references any of the given operator keys or aliases.
pub fn compiled_plan_has_operator(compiled_plan: &Value, targets: &[&str]) -> bool {
    let plan_name = compiled_plan
        .get("plan_name")
        .and_then(Value::as_str)
        .unwrap_or("");
    if targets
        .iter()
        .any(|t| !t.is_empty() && plan_name.contains(t))
    {
        return true;
    }
    let steps = compiled_plan
        .get("execution_contract")
        .and_then(|v| v.get("steps"))
        .and_then(|v| v.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[]);
    steps.iter().any(|step| {
        step.get("operator_key")
            .and_then(Value::as_str)
            .map(|ok| targets.iter().any(|t| ok == *t))
            .unwrap_or(false)
    })
}
