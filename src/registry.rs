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
    /// One-sentence Chinese description of what this operator does.
    pub description_zh: &'static str,
    /// Default parameter values as a JSON object: {"param": default_value, ...}
    pub default_params: Value,
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
            "description_zh": self.description_zh,
            "default_params": self.default_params,
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
            output_fields_zh: zh!["ATR值", "ATR上轨", "ATR下轨", "突破信号"],
            description_zh: "ATR平均真实波幅指标，计算ATR值及上下轨，当收盘价突破上轨或下轨时生成突破信号",
            default_params: json!({"fast_window_days": 14, "slow_window_days": 20}),
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
            description_zh: "布林带指标，计算中轨(MA20)、上下轨(±2σ)、带宽及扩张/收缩阶段判断",
            default_params: json!({"boll_period": 20, "boll_std_multiplier": 2.0}),
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
            description_zh: "布林上轨见顶出场信号，当价格上穿布林上轨后回落至中轨以下时触发卖出",
            default_params: json!({}),
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
            description_zh: "按信号强度排序分配仓位，每日最多N个买入信号，按强度加权分配预算比例",
            default_params: json!({"max_daily_buys": 5, "daily_buy_budget_ratio": 0.9, "target_position_per_trade": 0.12, "max_total_position_ratio": 1.0}),
        },
        OperatorEntry {
            operator_key: "ema5_ema10_adhesion_signal",
            operator_type: "indicator",
            execution_model: "columnar",
            aliases: &["ema5_ema10_adhesion_signal"],
            dependencies: &[],
            output_columns: &["adhesion_signal", "adhesion_strength", "ema5_value", "ema10_value", "gap_ratio"],
            input_fields_zh: zh!["价格", "成交量"],
            output_fields_zh: zh!["黏合信号", "黏合强度", "EMA5值", "EMA10值", "间隙比率"],
            description_zh: "EMA5-EMA10黏合指标，计算快慢EMA差值比率(gap_ratio)并输出黏合信号",
            default_params: json!({"fast_window_days": 5, "slow_window_days": 10}),
        },
        OperatorEntry {
            operator_key: "ema_slope_v1",
            operator_type: "indicator",
            execution_model: "columnar",
            aliases: &["ema_slope"],
            dependencies: &[],
            output_columns: &["ema5", "ema10", "ema20", "sma20", "ma20_slope", "ma20_slope_ratio", "ema5_ema20_diff"],
            input_fields_zh: zh!["价格"],
            output_fields_zh: zh!["EMA5", "EMA10", "EMA20", "SMA20", "MA20斜率", "MA20斜率比", "EMA5-EMA20差值"],
            description_zh: "EMA斜率指标，计算EMA5/10/20、SMA20、MA20斜率(5周期变化率)及EMA5-EMA20差值",
            default_params: json!({"ma20_slope_lookback": 5}),
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
            description_zh: "回测结束清仓，在每只标的最后一根K线处发出清仓卖出信号，用于回测强制平仓",
            default_params: json!({}),
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
            description_zh: "历史量能入场信号，检测均线上涨配合放量和阳线形态时生成入场信号",
            default_params: json!({}),
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
            description_zh: "历史量能门控，当成交量超过移动平均的一定倍数时生成量能通过/阻挡信号",
            default_params: json!({"volume_gate_ratio": 1.5, "volume_gate_lookback": 20}),
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
            description_zh: "K线数据输入算子，直接透传K线数据集并统计总品种数和总K线条数",
            default_params: json!({}),
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
            description_zh: "低位回补入场，检测价格接近布林下轨且带有长下影线的反弹K线，发出买入信号",
            default_params: json!({}),
        },
        OperatorEntry {
            operator_key: "ma20_entry_signal_v1",
            operator_type: "signal",
            execution_model: "columnar",
            aliases: &["ma20_entry_signal"],
            dependencies: &["bollinger_bands_v1", "ema_slope_v1", "volume_profile_v1"],
            output_columns: &["entry_signal", "entry_strength", "entry_price"],
            input_fields_zh: zh!["价格", "成交量", "布林带", "EMA斜率", "成交量概况"],
            output_fields_zh: zh!["入场信号", "入场强度", "入场价格"],
            description_zh: "基于布林下轨反弹、EMA回踩和强吸筹三种形态的MA20入场信号检测",
            default_params: json!({}),
        },
        OperatorEntry {
            operator_key: "ma20_exit_signal_v1",
            operator_type: "signal",
            execution_model: "columnar",
            aliases: &["ma20_exit_signal"],
            dependencies: &["bollinger_bands_v1", "ema_slope_v1"],
            output_columns: &["exit_signal", "exit_reason"],
            input_fields_zh: zh!["价格", "布林带", "EMA斜率"],
            output_fields_zh: zh!["出场信号", "出场原因"],
            description_zh: "基于趋势破坏和布林上轨假突破的MA20出场信号，含可配置止盈阈值和最大持仓天数",
            default_params: json!({"profit_take_threshold": 0.03, "max_holding_trading_days": 5}),
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
            description_zh: "趋势破坏出场，当收盘价跌破EMA20、EMA20斜率为负且K线收阴时发出卖出信号",
            default_params: json!({}),
        },
        OperatorEntry {
            operator_key: "ma_cross_v1",
            operator_type: "signal",
            execution_model: "columnar",
            aliases: &["ma_cross"],
            dependencies: &[],
            output_columns: &["cross_signal", "cross_strength", "fast_ma", "slow_ma"],
            input_fields_zh: zh!["价格"],
            output_fields_zh: zh!["交叉信号", "交叉强度", "快线MA", "慢线MA"],
            description_zh: "均线交叉信号，基于快慢EMA的金叉(买入)和死叉(卖出)判断趋势转变",
            default_params: json!({"fast_window_days": 5, "slow_window_days": 20, "threshold": 0.01}),
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
            description_zh: "主升浪见顶退出，基于长上影线比例、成交量放大和阴线形态判断顶部衰竭",
            default_params: json!({"mw_exit_upper_shadow_ratio": 0.6, "mw_exit_volume_spike_ratio": 2.0, "mw_exit_require_bearish": true, "mw_exit_cooldown_bars": 20}),
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
            description_zh: "最大持仓天数到期出场，每根K线携带可配置的最大持仓天数，超时强制平仓",
            default_params: json!({"max_holding_trading_days": 5}),
        },
        OperatorEntry {
            operator_key: "position_manager_v1",
            operator_type: "execution",
            execution_model: "columnar",
            aliases: &["position_manager"],
            dependencies: &[],
            output_columns: &["orders", "fills", "backtest_report", "position_summary", "market_risk_state"],
            input_fields_zh: zh!["交易信号列表"],
            output_fields_zh: zh!["订单", "成交", "回测报告", "持仓摘要", "市场风险状态"],
            description_zh: "多信号源汇聚与组合仓位管理，合并上游信号、执行下单成交撮合、持仓管理并生成回测报告",
            default_params: json!({"target_position_per_trade": 0.12, "max_daily_filled_symbols": 1}),
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
            description_zh: "止盈出场信号，每根K线携带可配置的止盈百分比阈值，供下游触发止盈判断",
            default_params: json!({"profit_take_threshold": 0.03}),
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
            description_zh: "16周期红绿占比指标，计算指定回看周期内阳线和阴线的占比",
            default_params: json!({"red_green_ratio_lookback": 16}),
        },
        OperatorEntry {
            operator_key: "red_green_ratio_signal_v1",
            operator_type: "signal",
            execution_model: "columnar",
            aliases: &["red_green_ratio_signal"],
            dependencies: &["red_green_ratio_16_v1"],
            output_columns: &["bull_signal", "bear_signal"],
            input_fields_zh: zh!["K线数据", "红绿比率"],
            output_fields_zh: zh!["多头信号", "空头信号"],
            description_zh: "红绿比率信号，当阳线占比超过阈值时产生多头信号，阴线占比超过阈值时产生空头信号",
            default_params: json!({"red_green_ratio_lookback": 16, "green_ratio_threshold": 0.6, "red_ratio_threshold": 0.6}),
        },
        OperatorEntry {
            operator_key: "rsi_reversal_v1",
            operator_type: "signal",
            execution_model: "columnar",
            aliases: &["rsi_14"],
            dependencies: &[],
            output_columns: &["rsi_signal", "rsi_value"],
            input_fields_zh: zh!["价格"],
            output_fields_zh: zh!["RSI反转信号", "RSI值"],
            description_zh: "RSI反转信号，基于快速RSI的超买(>70)超卖(<30)区域判断反转机会",
            default_params: json!({"fast_window_days": 3, "slow_window_days": 15, "threshold": 30.0}),
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
            description_zh: "强吸筹入场，检测EMA20正斜率、成交量放大(>1.5倍均量)、价格在布林中轨与上轨之间的吸筹形态",
            default_params: json!({}),
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
            description_zh: "成交量剖面指标，计算N周期均量和量比(当前成交量/均量)",
            default_params: json!({"volume_profile_lookback": 20}),
        },
        OperatorEntry {
            operator_key: "volume_trigger_v1",
            operator_type: "indicator",
            execution_model: "columnar",
            aliases: &["volume_trigger"],
            dependencies: &[],
            output_columns: &["volume_ratio", "volume_trigger", "trigger_fast_ratio", "trigger_slow_ratio", "volume_trigger_long", "volume_trigger_short"],
            input_fields_zh: zh!["成交量"],
            output_fields_zh: zh!["量比", "量触发信号", "快窗量比", "慢窗量比", "量能多头", "量能空头"],
            description_zh: "量能触发指标，基于日最大成交量双窗口基线(快窗均值+慢窗极值)和6-bar价格方向，检测放量突破/跌破信号",
            default_params: json!({
                "trigger_fast_threshold": 0.5,
                "trigger_slow_threshold": 2.0,
                "trigger_fast_window_days": 3,
                "trigger_slow_window_days": 15,
                "direction_lookback_bars": 6,
                "daily_ma120_ma200_gate_enabled": true,
                "cascade_direction_enabled": false,
                "cascade_direction_fast_window": 5,
                "cascade_direction_slow_window": 20,
                "bars_per_day": 16,
                "t_plus_one": true
            }),
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
            description_zh: "窗口K线涨跌统计，计算快慢两个窗口内的上涨/下跌根数及窗口首末收盘价",
            default_params: json!({"fast_window_days": 3, "slow_window_days": 15}),
        },
        OperatorEntry {
            operator_key: "window_up_buy_signal_v1",
            operator_type: "signal",
            execution_model: "columnar",
            aliases: &["window_up_buy_signal"],
            dependencies: &["window_kline_updown_stats_v1"],
            output_columns: &["buy_signal", "up_down_ratio", "price_change_pct"],
            input_fields_zh: zh!["K线数据", "窗口涨跌统计"],
            output_fields_zh: zh!["买入信号", "涨跌比", "窗口涨幅"],
            description_zh: "窗口上涨买入信号，当快窗口上涨根数超过慢窗口下跌根数且涨幅超过阈值时生成买入信号",
            default_params: json!({"fast_window_days": 3, "slow_window_days": 15, "threshold": 0.01}),
        },
        OperatorEntry {
            operator_key: "ma20_composite_v1",
            operator_type: "strategy",
            execution_model: "columnar",
            aliases: &["ma20_composite"],
            dependencies: &["ma20_entry_signal_v1", "ma20_exit_signal_v1", "position_manager_v1"],
            output_columns: &["signals", "orders", "fills", "backtest_report"],
            input_fields_zh: zh!["价格", "成交量"],
            output_fields_zh: zh!["交易信号", "订单", "成交", "回测报告"],
            description_zh: "MA20组合策略，编排ma20_entry_signal→ma20_exit_signal→position_manager三级流水线，产出完整交易闭环",
            default_params: json!({"target_position_per_trade": 0.12, "max_daily_buys": 1, "profit_take_threshold": 0.03}),
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
            description_zh: "级联过滤网关，通过方向过滤、成交量确认、积累判断和入场信号四重过滤生成交易信号",
            default_params: json!({"fast_window_days": 5, "slow_window_days": 20, "volume_trigger_ratio": 2.0, "volume_lookback_bars": 20, "cooldown_bars": 20}),
        },
        OperatorEntry {
            operator_key: "ema_cluster_breakout_v1",
            operator_type: "strategy",
            execution_model: "bars",
            aliases: &["ema_cluster_breakout"],
            dependencies: &[],
            output_columns: &["ema_cluster_breakout_signal", "signals"],
            input_fields_zh: zh!["价格", "成交量", "日K线"],
            output_fields_zh: zh!["EMA突破信号", "策略信号"],
            description_zh: "EMA均线簇突破策略，检测价格突破密集EMA均线区域(20/30/60/90/120)时生成买入信号",
            default_params: json!({
                "ema_cluster_breakout_enabled": true,
                "ema_cluster_breakout_periods": [20, 30, 60, 90, 120],
                "ema_cluster_breakout_density_threshold": 0.03,
                "ema_cluster_breakout_min_cross_count": 3,
                "ema_cluster_breakout_volume_ratio": 1.5,
                "ema_cluster_breakout_boll_width_expand_ratio": 1.05
            }),
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
            description_zh: "主升浪延续确认，在EMA突破后验证价格继续上涨趋势，确认进入主升浪",
            default_params: json!({"main_wave_verify_forward_bars": 80, "main_wave_verify_min_forward_return": 0.08, "main_wave_verify_max_pullback_ratio": 0.05}),
        },
        OperatorEntry {
            operator_key: "main_wave_verifier_v1",
            operator_type: "indicator",
            execution_model: "bars",
            aliases: &["main_wave_verifier"],
            dependencies: &[],
            output_columns: &["main_wave_verification", "verification_details"],
            input_fields_zh: zh!["日K线"],
            output_fields_zh: zh!["主升浪验证结果", "验证详情"],
            description_zh: "主升浪验证器，确认EMA突破后是否形成有效主升浪走势，含回撤容忍和位置风控",
            default_params: json!({
                "main_wave_verify_forward_bars": 80,
                "main_wave_verify_min_forward_return": 0.08,
                "main_wave_verify_max_pullback_ratio": 0.05,
                "main_wave_verify_require_shrink_retest": true,
                "main_wave_verify_position_gate_enabled": true
            }),
        },
        OperatorEntry {
            operator_key: "ma20_slope_bollinger_scale_v1",
            operator_type: "strategy",
            execution_model: "bars",
            aliases: &["ma20_slope_bollinger_scale"],
            dependencies: &[],
            output_columns: &["signals", "orders", "fills", "backtest_report", "strategy_audit", "position_summary"],
            input_fields_zh: zh!["价格", "成交量", "时间"],
            output_fields_zh: zh!["交易信号", "订单", "成交记录", "回测报告", "策略审计", "持仓摘要"],
            description_zh: "MA20主升浪完整策略，通过EMA斜率+布林带+成交量触发多信号入场，含止盈止损和主升浪持仓管理",
            default_params: json!({
                "target_position_per_trade": 0.12,
                "stop_loss_pct": 0.08,
                "profit_take_threshold": 0.03,
                "max_holding_trading_days": 2,
                "max_daily_buys": 1,
                "max_total_position_ratio": 1.0,
                "cooldown_bars": 0,
                "daily_ma120_ma200_buy_gate_enabled": true,
                "cascade_direction_enabled": false,
                "cascade_volume_enabled": false
            }),
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
            description_zh: "尾盘低位反弹策略，尾盘窗口(14:30-15:00)检测价格接近日低且EMA上行放量时入场，布林上轨长上影或止损出场",
            default_params: json!({
                "tail_start_time": "14:30",
                "tail_end_time": "15:00",
                "stop_loss_pct": 0.05,
                "max_holding_trading_days": 3,
                "boll_period": 20,
                "boll_std_multiplier": 2.0,
                "quantity_ratio": 0.12,
                "max_daily_buys": 1,
                "max_total_position_ratio": 1.0,
                "cascade_direction_enabled": true,
                "cascade_volume_enabled": true,
                "cascade_volume_ratio": 2.0,
                "volume_lookback_bars": 20,
                "daily_ma120_ma200_buy_gate_enabled": false
            }),
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
