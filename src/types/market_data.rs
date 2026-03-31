use chrono::{DateTime, NaiveDate, Utc};
use pretty_simple_display::{DebugPretty, DisplaySimple};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{InstrumentType, Symbol};

#[derive(DebugPretty, DisplaySimple, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MarketData {
    pub symbol: Symbol,
    pub instrument_type: InstrumentType,
    pub updated_at: DateTime<Utc>,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    #[serde(default)]
    pub bid: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    #[serde(default)]
    pub bid_size: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    #[serde(default)]
    pub ask: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    #[serde(default)]
    pub ask_size: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    #[serde(default)]
    pub mid: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    #[serde(default)]
    pub mark: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    #[serde(default)]
    pub last: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    #[serde(default)]
    pub last_mkt: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    #[serde(default)]
    pub beta: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    #[serde(default)]
    pub dividend_amount: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    #[serde(default)]
    pub dividend_frequency: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub open: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub day_high_price: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub day_low_price: Decimal,
    pub close_price_type: String,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub prev_close: Decimal,
    pub prev_close_price_type: String,
    pub prev_close_date: NaiveDate,
    pub summary_date: NaiveDate,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    #[serde(default)]
    pub low_limit_price: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    #[serde(default)]
    pub high_limit_price: Decimal,
    pub is_trading_halted: bool,
    #[serde(default)]
    pub halt_start_time: i64,
    #[serde(default)]
    pub halt_end_time: i64,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub year_low_price: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub year_high_price: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    #[serde(default)]
    pub volume: Decimal,
}
