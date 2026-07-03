use serde_json::Value;

// —— Core data types ——

/// A single OHLCV bar, shared between backtest and signal runtime.
#[derive(Debug, Clone)]
pub struct Bar {
    pub symbol: String,
    pub ts_utc: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

// —— Operator metadata ——

#[derive(Debug, Clone)]
pub struct OperatorMeta {
    pub operator_key: &'static str,
    pub operator_type: &'static str,
    pub aliases: &'static [&'static str],
    pub dependencies: &'static [&'static str],
    pub output_columns: &'static [&'static str],
    pub input_fields_zh: Value,
    pub output_fields_zh: Value,
}

impl OperatorMeta {
    pub fn to_capability(
        &self,
        contract_version: &str,
        execution_model: &str,
        volume_unit: &str,
        required_columns: &[&str],
        optional_columns: &[&str],
    ) -> Value {
        serde_json::json!({
            "operator_key": self.operator_key,
            "operator_type": self.operator_type,
            "runtime": "rust",
            "status": "stable",
            "input_contract": {
                "contract_version": contract_version,
                "execution_model": execution_model,
                "volume_unit": volume_unit,
                "legacy_row_adapter_used": false,
                "depends_on": self.dependencies,
                "required_columns": required_columns,
                "optional_columns": optional_columns,
            },
            "output_columns": self.output_columns,
            "input_fields_zh": self.input_fields_zh,
            "output_schema": {
                "fields_zh": self.output_fields_zh,
            },
        })
    }
}

// —— Operator traits ——

/// Operator that processes columnar (Arrow-based) K-line frames.
pub trait ColumnarOperator: Send + Sync {
    fn metadata(&self) -> &OperatorMeta;
    fn run(&self, parameters: &Value) -> Value;
}

/// Operator that processes per-bar (row-based) K-line data.
pub trait BarsOperator: Send + Sync {
    fn metadata(&self) -> &OperatorMeta;
    fn run(&self, bars: &[Bar], parameters: &Value) -> Value;
    fn run_with_daily(&self, bars: &[Bar], _daily_bars: &[Bar], parameters: &Value) -> Value {
        self.run(bars, parameters)
    }
}
