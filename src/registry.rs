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
    /// Per-parameter Chinese descriptions: {"param_name": "中文描述", ...}
    pub param_descriptions_zh: Value,
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
            "param_descriptions_zh": self.param_descriptions_zh,
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
            description_zh: "ATR真实波幅突破指标，价格波动率放大、突破ATR上下轨时产生交易信号",
            default_params: json!({"fast_window_days": 14, "slow_window_days": 20}),
            param_descriptions_zh: json!({"fast_window_days": "快窗口周期（日）","slow_window_days": "慢窗口周期（日）"}),
        },
        OperatorEntry {
            operator_key: "bollinger_bands_v1",
            operator_type: "indicator",
            execution_model: "columnar",
            aliases: &["bollinger_bands"],
            dependencies: &[],
            output_columns: &[
                "boll_mid",
                "boll_upper",
                "boll_lower",
                "boll_width",
                "boll_phase",
            ],
            input_fields_zh: zh!["价格"],
            output_fields_zh: zh!["布林中轨", "布林上轨", "布林下轨", "布林带宽", "布林阶段"],
            description_zh: "布林带通道指标，价格围绕MA20中轨波动，上轨=中轨+2倍标准差（压力位），下轨=中轨-2倍标准差（支撑位），带宽扩张=趋势启动，收缩=变盘前兆",
            default_params: json!({"boll_period": 20, "boll_std_multiplier": 2.0}),
            param_descriptions_zh: json!({"boll_period": "布林带周期","boll_std_multiplier": "标准差倍数"}),
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
            description_zh: "布林带上轨见顶出场，价格冲高到布林上轨后回落跌破中轨时触发卖出",
            default_params: json!({"boll_period": 20, "boll_std_multiplier": 2.0, "require_bearish_candle": true}),
            param_descriptions_zh: json!({"boll_period": "布林带周期","boll_std_multiplier": "标准差倍数","require_bearish_candle": "要求阴线确认"}),
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
            description_zh: "仓位分配器，按信号强度排序分配每日买入额度，强度越高仓位越大",
            default_params: json!({"max_daily_buys": 5, "daily_buy_budget_ratio": 0.9, "target_position_per_trade": 0.12, "max_total_position_ratio": 1.0}),
            param_descriptions_zh: json!({"max_daily_buys": "每日最大买入数","daily_buy_budget_ratio": "每日买入预算比例","target_position_per_trade": "单笔目标仓位比例","max_total_position_ratio": "最大总仓位比例"}),
        },
        OperatorEntry {
            operator_key: "ema5_ema10_adhesion_signal",
            operator_type: "indicator",
            execution_model: "columnar",
            aliases: &["ema5_ema10_adhesion_signal"],
            dependencies: &[],
            output_columns: &[
                "adhesion_signal",
                "adhesion_strength",
                "ema5_value",
                "ema10_value",
                "gap_ratio",
            ],
            input_fields_zh: zh!["价格", "成交量"],
            output_fields_zh: zh!["黏合信号", "黏合强度", "EMA5值", "EMA10值", "间隙比率"],
            description_zh: "EMA均线黏合检测，快慢EMA接近缠绕时发出黏合信号，预示均线即将发散",
            default_params: json!({"fast_window_days": 5, "slow_window_days": 10}),
            param_descriptions_zh: json!({"fast_window_days": "快窗口周期（日）","slow_window_days": "慢窗口周期（日）"}),
        },
        OperatorEntry {
            operator_key: "ema_slope_v1",
            operator_type: "indicator",
            execution_model: "columnar",
            aliases: &["ema_slope"],
            dependencies: &[],
            output_columns: &[
                "ema5",
                "ema10",
                "ema20",
                "sma20",
                "ma20_slope",
                "ma20_slope_ratio",
                "ema5_ema20_diff",
            ],
            input_fields_zh: zh!["价格"],
            output_fields_zh: zh![
                "EMA5",
                "EMA10",
                "EMA20",
                "SMA20",
                "MA20斜率",
                "MA20斜率比",
                "EMA5-EMA20差值"
            ],
            description_zh: "EMA均线组指标，计算EMA5/10/20、SMA20、MA20斜率，判断均线排列方向和趋势强弱",
            default_params: json!({"ma20_slope_lookback": 5}),
            param_descriptions_zh: json!({"ma20_slope_lookback": "MA20斜率回看周期"}),
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
            param_descriptions_zh: json!({}),
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
            description_zh: "复合条件放量入场信号，需要均线上涨趋势+成交量放大+阳线三个条件同时满足才触发买入，不是单纯的放量突破检测（纯放量突破请用volume_trigger）",
            default_params: json!({"ma20_slope_min_ratio": 0.02, "vol_ratio_min": 1.2, "require_bullish_candle": true, "require_above_ema20": true}),
            param_descriptions_zh: json!({"ma20_slope_min_ratio": "MA20斜率最小比例","vol_ratio_min": "量比最小阈值","require_bullish_candle": "要求阳线","require_above_ema20": "要求站上EMA20"}),
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
            description_zh: "成交量阈值过滤器，当前成交量超过均量N倍时允许通过（可配置倍数和均量窗口），防止缩量行情中入场",
            default_params: json!({"volume_gate_ratio": 1.5, "volume_gate_lookback": 20}),
            param_descriptions_zh: json!({"volume_gate_ratio": "量能门控比率","volume_gate_lookback": "量能门控回看周期"}),
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
            param_descriptions_zh: json!({}),
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
            description_zh: "低位反弹抄底入场，价格跌到布林下轨附近出现长下影线反弹K线时买入",
            default_params: json!({"boll_period": 20, "boll_std_multiplier": 2.0, "near_boll_lower_threshold": 1.03, "lower_shadow_ratio_min": 0.3, "require_bullish_candle": true}),
            param_descriptions_zh: json!({"boll_period": "布林带周期","boll_std_multiplier": "标准差倍数","near_boll_lower_threshold": "接近下轨阈值","lower_shadow_ratio_min": "下影线最小比例","require_bullish_candle": "要求阳线"}),
        },
        OperatorEntry {
            operator_key: "ma20_entry_signal_v1",
            operator_type: "signal",
            execution_model: "columnar",
            aliases: &["ma20_entry_signal"],
            dependencies: &["bollinger_bands_v1", "ema_slope_v1", "volume_profile_v1", "volume_trigger_v1"],
            output_columns: &["entry_signal", "entry_strength", "entry_price"],
            input_fields_zh: zh!["价格", "成交量", "布林带", "EMA斜率", "成交量概况"],
            output_fields_zh: zh!["入场信号", "入场强度", "入场价格"],
            description_zh: "MA20入场信号检测器，识别三种买入形态：布林下轨反弹、EMA回踩反弹、强吸筹放量上涨",
            default_params: json!({"boll_lower_rebound_enabled": true, "ema_pullback_enabled": true, "strong_accumulation_enabled": true, "boll_lower_near_ratio": 1.02, "ema_pullback_premium_ratio": 1.03, "strong_acc_ma20_slope_min": 0.05, "ma_slope_min_ratio": 0.05, "strong_acc_vol_ratio_min": 1.5, "ema_pullback_vol_ratio_min": 0.8, "cooldown_bars": 20, "stock_volume_gate_enabled": false, "volume_trigger_ratio": 2.0, "volume_lookback_bars": 20, "boll_period": 20, "boll_std_multiplier": 2.0}),
            param_descriptions_zh: json!({"boll_lower_rebound_enabled": "启用布林下轨反弹","ema_pullback_enabled": "启用EMA回踩","strong_accumulation_enabled": "启用强吸筹","boll_lower_near_ratio": "布林下轨接近比例","ema_pullback_premium_ratio": "EMA回踩溢价比例","strong_acc_ma20_slope_min": "强吸筹MA20斜率最小值","ma_slope_min_ratio": "MA斜率最小比例","strong_acc_vol_ratio_min": "强吸筹量比最小值","ema_pullback_vol_ratio_min": "EMA回踩量比最小值","cooldown_bars": "冷却K线数","stock_volume_gate_enabled": "启用个股量能门控","volume_trigger_ratio": "量能触发比率","volume_lookback_bars": "量能回看K线数","boll_period": "布林带周期","boll_std_multiplier": "标准差倍数"}),
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
            description_zh: "MA20出场信号检测器，识别两种卖出形态：趋势破坏（跌破EMA20+斜率转负）和布林上轨假突破",
            default_params: json!({"profit_take_threshold": 0.03, "max_holding_trading_days": 5}),
            param_descriptions_zh: json!({"profit_take_threshold": "止盈阈值","max_holding_trading_days": "最大持仓交易日"}),
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
            description_zh: "趋势破坏出场，收盘价跌破EMA20+EMA20斜率转负+K线收阴，三条件同时满足时卖出",
            default_params: json!({"ma20_slope_max_ratio": -0.02, "require_bearish_candle": true, "require_below_ema20": true}),
            param_descriptions_zh: json!({"ma20_slope_max_ratio": "MA20斜率最大比例","require_bearish_candle": "要求阴线","require_below_ema20": "要求跌破EMA20"}),
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
            description_zh: "均线金叉死叉交易信号，快线(EMA5)上穿慢线(EMA20)=金叉买入，下穿=死叉卖出，经典双均线趋势跟踪策略",
            default_params: json!({"fast_window_days": 5, "slow_window_days": 20, "threshold": 0.01}),
            param_descriptions_zh: json!({"fast_window_days": "快窗口周期（日）","slow_window_days": "慢窗口周期（日）","threshold": "交叉阈值"}),
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
            description_zh: "主升浪顶部卖出，长上影线+放量+阴线组合判断主力出货，顶部衰竭时离场",
            default_params: json!({"mw_exit_upper_shadow_ratio": 0.6, "mw_exit_volume_spike_ratio": 2.0, "mw_exit_require_bearish": true, "mw_exit_cooldown_bars": 20}),
            param_descriptions_zh: json!({"mw_exit_upper_shadow_ratio": "上影线比例","mw_exit_volume_spike_ratio": "放量倍数","mw_exit_require_bearish": "要求阴线","mw_exit_cooldown_bars": "冷却K线数"}),
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
            param_descriptions_zh: json!({"max_holding_trading_days": "最大持仓交易日"}),
        },
        OperatorEntry {
            operator_key: "stop_loss_exit_v1",
            operator_type: "exit",
            execution_model: "columnar",
            aliases: &["rust_builtin.stop_loss_exit_v1"],
            dependencies: &[],
            output_columns: &["stop_loss_signal"],
            input_fields_zh: zh!["K线数据", "上游入场信号"],
            output_fields_zh: zh!["止损出场信号"],
            description_zh: "止损出场，接收上游入场信号，扫描入场后的K线找到第一根 low <= entry_price*(1-stop_loss_pct) 的bar发出场信号",
            default_params: json!({"stop_loss_pct": 0.08}),
            param_descriptions_zh: json!({"stop_loss_pct": "止损百分比"}),
        },
        OperatorEntry {
            operator_key: "trailing_stop_v1",
            operator_type: "exit",
            execution_model: "columnar",
            aliases: &["rust_builtin.trailing_stop_v1"],
            dependencies: &[],
            output_columns: &["trailing_stop_signal"],
            input_fields_zh: zh!["K线数据", "上游入场信号"],
            output_fields_zh: zh!["移动止盈出场信号"],
            description_zh: "移动止盈出场，接收上游入场信号，入场后跟踪最高价，从高点回落 trailing_stop_pct 时触发出场，动态锁定利润",
            default_params: json!({"trailing_stop_pct": 0.05}),
            param_descriptions_zh: json!({"trailing_stop_pct": "回撤止盈百分比（从最高点回落幅度）"}),
        },
        OperatorEntry {
            operator_key: "tail_low_rebound_entry_v1",
            operator_type: "entry",
            execution_model: "columnar",
            aliases: &["rust_builtin.tail_low_rebound_entry_v1"],
            dependencies: &[],
            output_columns: &["tail_low_rebound_entry_signal"],
            input_fields_zh: zh!["K线数据", "日线趋势门控结果", "级联方向门控结果"],
            output_fields_zh: zh!["尾盘低位反弹入场信号"],
            description_zh: "尾盘低位反弹入场检测，尾盘窗口(14:30-15:00)检测价格接近日低且EMA上行放量时发出入场信号",
            default_params: json!({"tail_start_time": "14:30", "tail_end_time": "15:00", "ma_slope_min_ratio": 0.001, "near_low_max_price_pct": 0.01, "near_low_max_day_range_ratio": 0.20, "volume_trigger_ratio": 2.0, "volume_lookback_bars": 20, "boll_period": 20, "boll_std_multiplier": 2.0, "cascade_direction_enabled": true, "cascade_volume_enabled": true, "daily_ma120_ma200_buy_gate_enabled": false}),
            param_descriptions_zh: json!({"tail_start_time": "尾盘开始时间","tail_end_time": "尾盘结束时间","ma_slope_min_ratio": "MA斜率最小比例","near_low_max_price_pct": "接近日低最大价格百分比","near_low_max_day_range_ratio": "接近日低最大日振幅比例","volume_trigger_ratio": "量能触发比率","volume_lookback_bars": "量能回看K线数","boll_period": "布林带周期","boll_std_multiplier": "标准差倍数","cascade_direction_enabled": "启用级联方向门控","cascade_volume_enabled": "启用级联量能门控","daily_ma120_ma200_buy_gate_enabled": "启用MA120/MA200买入门控"}),
        },
        OperatorEntry {
            operator_key: "position_manager_v1",
            operator_type: "execution",
            execution_model: "columnar",
            aliases: &["position_manager"],
            dependencies: &["ma20_entry_signal_v1", "ma20_exit_signal_v1"],
            output_columns: &[
                "orders",
                "fills",
                "backtest_report",
                "position_summary",
                "market_risk_state",
            ],
            input_fields_zh: zh!["交易信号列表"],
            output_fields_zh: zh!["订单", "成交", "回测报告", "持仓摘要", "市场风险状态"],
            description_zh: "多信号汇聚与组合仓位管理，合并上游信号、执行下单成交撮合、持仓管理并生成回测报告",
            default_params: json!({"target_position_per_trade": 0.12, "max_daily_filled_symbols": 1}),
            param_descriptions_zh: json!({"target_position_per_trade": "单笔目标仓位比例","max_daily_filled_symbols": "每日最大成交标的数"}),
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
            param_descriptions_zh: json!({"profit_take_threshold": "止盈阈值"}),
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
            description_zh: "K线红绿柱统计，计算N周期内阳线和阴线的数量和占比，判断多空力量对比",
            default_params: json!({"red_green_ratio_lookback": 16}),
            param_descriptions_zh: json!({"red_green_ratio_lookback": "红绿比率回看周期"}),
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
            description_zh: "红绿柱多空信号，阳线占比超过阈值=做多信号，阴线占比超过阈值=做空信号",
            default_params: json!({"red_green_ratio_lookback": 16, "green_ratio_threshold": 0.6, "red_ratio_threshold": 0.6}),
            param_descriptions_zh: json!({"red_green_ratio_lookback": "红绿比率回看周期","green_ratio_threshold": "阳线占比阈值","red_ratio_threshold": "阴线占比阈值"}),
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
            description_zh: "RSI超买超卖反转交易，RSI高于70=超买做空，低于30=超卖做多",
            default_params: json!({"fast_window_days": 3, "slow_window_days": 15, "threshold": 30.0}),
            param_descriptions_zh: json!({"fast_window_days": "快窗口周期（日）","slow_window_days": "慢窗口周期（日）","threshold": "RSI阈值"}),
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
            description_zh: "强吸筹买入信号，EMA20向上+成交量放大1.5倍以上+价格在布林中上轨之间，主力吸筹形态",
            default_params: json!({"ma20_slope_min_ratio": 0.05, "vol_ratio_min": 1.5, "require_between_boll_mid_upper": true}),
            param_descriptions_zh: json!({"ma20_slope_min_ratio": "MA20斜率最小比例","vol_ratio_min": "量比最小阈值","require_between_boll_mid_upper": "要求在布林中上轨间"}),
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
            description_zh: "成交量和量比计算器，输出N周期均量和当前量比（当前量/均量），用于判断放量还是缩量，常被其他策略作为成交量数据源引用",
            default_params: json!({"volume_profile_lookback": 20}),
            param_descriptions_zh: json!({"volume_profile_lookback": "量能概况回看周期"}),
        },
        OperatorEntry {
            operator_key: "volume_trigger_v1",
            operator_type: "indicator",
            execution_model: "columnar",
            aliases: &["volume_trigger"],
            dependencies: &[],
            output_columns: &[
                "volume_ratio",
                "volume_trigger",
                "trigger_fast_ratio",
                "trigger_slow_ratio",
                "volume_trigger_long",
                "volume_trigger_short",
            ],
            input_fields_zh: zh!["成交量"],
            output_fields_zh: zh![
                "量比",
                "量触发信号",
                "快窗量比",
                "慢窗量比",
                "量能多头",
                "量能空头"
            ],
            description_zh: "放量突破检测器，专门检测成交量突然放大突破的形态。当前成交量超过历史均量N倍时判定为放量突破（典型的放量突破场景：成交量放大超过20日均量2倍），生成放量买入/卖出信号",
            default_params: json!({
                "fast_threshold": 0.5,
                "slow_threshold": 2.0,
                "fast_window_days": 3,
                "slow_window_days": 15,
                "direction_lookback_bars": 6,
                "daily_ma120_ma200_gate_enabled": true,
                "cascade_direction_enabled": false,
                "cascade_direction_fast_window": 5,
                "cascade_direction_slow_window": 20,
                "bars_per_day": 16,
                "t_plus_one": true
            }),
            param_descriptions_zh: json!({"fast_threshold": "快窗量比阈值","slow_threshold": "慢窗量比阈值","fast_window_days": "快窗口周期（日）","slow_window_days": "慢窗口周期（日）","direction_lookback_bars": "方向回看K线数","daily_ma120_ma200_gate_enabled": "启用日线MA120/MA200门控","cascade_direction_enabled": "启用级联方向","cascade_direction_fast_window": "级联方向快窗","cascade_direction_slow_window": "级联方向慢窗","bars_per_day": "每日K线数","t_plus_one": "T+1交易制度"}),
        },
        OperatorEntry {
            operator_key: "window_kline_updown_stats_v1",
            operator_type: "indicator",
            execution_model: "columnar",
            aliases: &["window_kline_updown_stats"],
            dependencies: &[],
            output_columns: &["up_count", "down_count", "first_close", "last_close"],
            input_fields_zh: zh!["K线数据"],
            output_fields_zh: zh![
                "快窗口上涨根数",
                "慢窗口下跌根数",
                "快窗口首根收盘价",
                "快窗口末根收盘价"
            ],
            description_zh: "窗口K线涨跌统计，统计短期和长期两个窗口内阳线/阴线根数及首末收盘价，用于判断短期动能方向和长期趋势的背离",
            default_params: json!({"fast_window_days": 3, "slow_window_days": 15}),
            param_descriptions_zh: json!({"fast_window_days": "快窗口周期（日）","slow_window_days": "慢窗口周期（日）"}),
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
            description_zh: "窗口动能买入信号，短期上涨力度超过长期下跌力度且窗口涨幅达标时触发买入，经典的短多压过长空的动量交易信号",
            default_params: json!({"fast_window_days": 3, "slow_window_days": 15, "threshold": 0.01}),
            param_descriptions_zh: json!({"fast_window_days": "快窗口周期（日）","slow_window_days": "慢窗口周期（日）","threshold": "信号阈值"}),
        },
        OperatorEntry {
            operator_key: "ma20_composite_v1",
            operator_type: "strategy",
            execution_model: "columnar",
            aliases: &["ma20_composite"],
            dependencies: &[
                "ma20_entry_signal_v1",
                "ma20_exit_signal_v1",
                "position_manager_v1",
            ],
            output_columns: &["signals", "orders", "fills", "backtest_report"],
            input_fields_zh: zh!["价格", "成交量"],
            output_fields_zh: zh!["交易信号", "订单", "成交", "回测报告"],
            description_zh: "MA20均线组合交易策略，基于MA20均线的完整买卖体系：MA20附近入场（下轨反弹/回踩/放量上涨）→MA20趋势破坏或止盈出场→自动仓位管理，适合趋势跟踪交易",
            default_params: json!({"target_position_per_trade": 0.12, "max_daily_buys": 1, "profit_take_threshold": 0.03}),
            param_descriptions_zh: json!({"target_position_per_trade": "单笔目标仓位比例","max_daily_buys": "每日最大买入数","profit_take_threshold": "止盈阈值"}),
        },
        OperatorEntry {
            operator_key: "daily_trend_gate_v1",
            operator_type: "gate",
            execution_model: "columnar",
            aliases: &["daily_trend_gate"],
            dependencies: &[],
            output_columns: &[],
            input_fields_zh: zh!["日线K线"],
            output_fields_zh: zh!["日线趋势信号", "日线方向信号"],
            description_zh: "日线趋势门控，计算EMA快慢线趋势方向（MA120/MA200）和EMA快慢线方向（日线级联方向），输出按日期×标的的趋势映射",
            default_params: json!({"trend_fast": 120, "trend_slow": 200, "direction_fast": 3, "direction_slow": 15}),
            param_descriptions_zh: json!({"trend_fast": "趋势快线周期","trend_slow": "趋势慢线周期","direction_fast": "方向快线周期","direction_slow": "方向慢线周期"}),
        },
        OperatorEntry {
            operator_key: "regime_calendar_gate_v1",
            operator_type: "gate",
            execution_model: "columnar",
            aliases: &["regime_calendar_gate"],
            dependencies: &[],
            output_columns: &["regime_state"],
            input_fields_zh: zh!["K线数据", "震荡区间日历参数"],
            output_fields_zh: zh!["市场状态信号(1=趋势,0=震荡)"],
            description_zh: "大盘状态日历门控：按先验固定的震荡区间日历（离线由指数日线规则预计算）逐bar输出 regime_state 列，1.0=趋势态、0.0=震荡态，供出场/仓位算子做分段参数切换",
            default_params: json!({"osc_date_ranges": []}),
            param_descriptions_zh: json!({"osc_date_ranges": "震荡区间日历，[[起始日,结束日],...] 格式，YYYY-MM-DD"}),
        },
        OperatorEntry {
            operator_key: "regime_index_gate_v1",
            operator_type: "gate",
            execution_model: "columnar",
            aliases: &["regime_index_gate"],
            dependencies: &[],
            output_columns: &["regime_state", "regime_entry_allow"],
            input_fields_zh: zh!["K线数据", "指数日线帧(IDX_前缀)"],
            output_fields_zh: zh!["市场状态信号(1=趋势,0=震荡)", "入场缩仓系数(1=全仓买入,0=禁止,0~1=降仓)"],
            description_zh: "大盘状态在线门控(三态)：从数据集内指数日线帧(IDX_前缀符号)在线计算 close>MA(N) 且 MA 上行(slope_lag日) + confirm_days 日迟滞 + D+1 生效，逐bar输出 regime_state 列(1.0=趋势/0.0=震荡)与 regime_entry_allow 列——趋势态与上行震荡态(大盘momentum_lag日动量≥0)全仓买入(1.0)；下跌震荡态(动量<0)输出 deny_scale(默认0=禁买，0.5=半仓)，仅当大盘前一交易日收盘跌破布林下轨(boll_len,boll_k)超卖溢出时全仓放行；数据集需含指数符号(使用 *_idx dataset)",
            default_params: json!({"index_symbol": "IDX_SH000001", "ma_len": 20, "slope_lag": 5, "confirm_days": 3, "boll_len": 20, "boll_k": 2.0, "momentum_lag": 20, "deny_scale": 0.0}),
            param_descriptions_zh: json!({"index_symbol": "指数符号(IDX_前缀，需存在于数据集)", "ma_len": "均线周期", "slope_lag": "均线斜率回看交易日数", "confirm_days": "状态翻转连续确认天数(迟滞)", "boll_len": "大盘布林带周期(入场例外判定)", "boll_k": "大盘布林带标准差倍数(入场例外判定)", "momentum_lag": "大盘动量回看交易日数(区分上行/下跌震荡)", "deny_scale": "下跌震荡态入场缩仓系数(0=禁买,0.5=半仓,1=不门控)"}),
        },
        // —— Bars operators ——
        OperatorEntry {
            operator_key: "cascade_gate_v1",
            operator_type: "gate",
            execution_model: "columnar",
            aliases: &["cascade_gate"],
            dependencies: &["daily_trend_gate_v1"],
            output_columns: &[],
            input_fields_zh: zh!["15分钟K线", "日线趋势数据"],
            output_fields_zh: zh!["级联过滤信号"],
            description_zh: "级联过滤网关，基于日线趋势方向、成交量确认、入场信号检测和冷却机制生成交易信号",
            default_params: json!({"volume_trigger_ratio": 2.0, "volume_lookback_bars": 20, "cooldown_bars": 20}),
            param_descriptions_zh: json!({"volume_trigger_ratio": "量能触发比率","volume_lookback_bars": "量能回看K线数","cooldown_bars": "冷却K线数"}),
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
            description_zh: "EMA均线簇突破买入，价格从下方上穿密集的EMA均线群(20/30/60/90/120日均线)且放量确认时触发买入，均线密集后发散是主升浪启动信号",
            default_params: json!({
                "ema_cluster_breakout_enabled": true,
                "ema_cluster_breakout_periods": [20, 30, 60, 90, 120],
                "ema_cluster_breakout_density_threshold": 0.03,
                "ema_cluster_breakout_min_cross_count": 3,
                "ema_cluster_breakout_volume_ratio": 1.5,
                "ema_cluster_breakout_boll_width_expand_ratio": 1.05
            }),
            param_descriptions_zh: json!({"ema_cluster_breakout_enabled": "启用EMA簇突破","ema_cluster_breakout_periods": "EMA周期列表","ema_cluster_breakout_density_threshold": "均线密集度阈值","ema_cluster_breakout_min_cross_count": "最小穿越均线数","ema_cluster_breakout_volume_ratio": "放量确认比率","ema_cluster_breakout_boll_width_expand_ratio": "布林带宽扩张比率"}),
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
            description_zh: "主升浪延续买入信号，EMA均线突破后向前验证一定周期内的涨幅是否达标（可配置验证周期和最低涨幅），确认不是假突破后入场做多",
            default_params: json!({"main_wave_verify_forward_bars": 80, "main_wave_verify_min_forward_return": 0.08, "main_wave_verify_max_pullback_ratio": 0.05}),
            param_descriptions_zh: json!({"main_wave_verify_forward_bars": "前向验证K线数","main_wave_verify_min_forward_return": "最小前向收益率","main_wave_verify_max_pullback_ratio": "最大回撤容忍比例"}),
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
            description_zh: "主升浪真假突破验证，EMA均线突破后验证后续走势：涨幅是否达标、回撤是否在容忍范围内、当前位置是否过高，用于过滤假突破",
            default_params: json!({
                "main_wave_verify_forward_bars": 80,
                "main_wave_verify_min_forward_return": 0.08,
                "main_wave_verify_max_pullback_ratio": 0.05,
                "main_wave_verify_require_shrink_retest": true,
                "main_wave_verify_position_gate_enabled": true
            }),
            param_descriptions_zh: json!({"main_wave_verify_forward_bars": "前向验证K线数","main_wave_verify_min_forward_return": "最小前向收益率","main_wave_verify_max_pullback_ratio": "最大回撤容忍比例","main_wave_verify_require_shrink_retest": "要求缩量回踩确认","main_wave_verify_position_gate_enabled": "启用位置门控"}),
        },
        OperatorEntry {
            operator_key: "ma20_slope_bollinger_scale_v1",
            operator_type: "strategy",
            execution_model: "bars",
            aliases: &["ma20_slope_bollinger_scale"],
            dependencies: &[],
            output_columns: &[
                "signals",
                "orders",
                "fills",
                "backtest_report",
                "strategy_audit",
                "position_summary",
            ],
            input_fields_zh: zh!["价格", "成交量", "时间"],
            output_fields_zh: zh![
                "交易信号",
                "订单",
                "成交记录",
                "回测报告",
                "策略审计",
                "持仓摘要"
            ],
            description_zh: "MA20主升浪完整交易策略，价格沿MA20均线持续上涨时做多，含布林带波动率自适应仓位、止盈止损和主升浪持仓管理",
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
            param_descriptions_zh: json!({"target_position_per_trade": "单笔目标仓位比例","stop_loss_pct": "止损百分比","profit_take_threshold": "止盈阈值","max_holding_trading_days": "最大持仓交易日","max_daily_buys": "每日最大买入数","max_total_position_ratio": "最大总仓位比例","cooldown_bars": "冷却K线数","daily_ma120_ma200_buy_gate_enabled": "启用MA120/MA200买入门控","cascade_direction_enabled": "启用级联方向","cascade_volume_enabled": "启用级联量能"}),
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
            param_descriptions_zh: json!({"tail_start_time": "尾盘开始时间","tail_end_time": "尾盘结束时间","stop_loss_pct": "止损百分比","max_holding_trading_days": "最大持仓交易日","boll_period": "布林带周期","boll_std_multiplier": "标准差倍数","quantity_ratio": "单笔仓位比例","max_daily_buys": "每日最大买入数","max_total_position_ratio": "最大总仓位比例","cascade_direction_enabled": "启用级联方向门控","cascade_volume_enabled": "启用级联量能门控","cascade_volume_ratio": "级联量能比率","volume_lookback_bars": "量能回看K线数","daily_ma120_ma200_buy_gate_enabled": "启用MA120/MA200买入门控"}),
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
