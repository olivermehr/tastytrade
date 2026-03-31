use chrono::{DateTime, Utc};
use pretty_simple_display::{DebugPretty, DisplaySimple};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::Symbol;

/// Represents the option metrics for a given symbol.
///
/// This struct holds implied volatility, liquidity and individual option expiration metrics.

#[derive(DebugPretty, DisplaySimple, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct OptionMetrics {
    pub symbol: Symbol,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub corr_spy_3month: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub historical_volatility_30_day: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub historical_volatility_60_day: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub historical_volatility_90_day: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub implied_volatility_index: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub implied_volatility_index_rank: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub implied_volatility_percentile: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub liquidity_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub liquidity_rank: Decimal,
    pub liquidity_rating: u32,
    pub option_expiration_implied_volatilities: Vec<OptionExpirationImpliedVolatility>,
    updated_at: DateTime<Utc>,
}

#[derive(DebugPretty, DisplaySimple, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct OptionExpirationImpliedVolatility {
    pub expiration_date: String,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    #[serde(default)]
    pub implied_volatility: Decimal,
    pub settlement_type: String,
    pub option_chain_type: String,
}