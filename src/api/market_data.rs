use crate::api::base::{Items};
use crate::{AsSymbol, TastyResult, TastyTrade, types::market_data::MarketData};
use crate::utils::join::join_symbols_optional;

#[derive(Default)]
pub struct MarketDataRequest<'a, T: AsSymbol> {
    pub index: Option<&'a [T]>,
    pub equity: Option<&'a [T]>,
    pub equity_option: Option<&'a [T]>,
    pub future: Option<&'a [T]>,
    pub future_option: Option<&'a [T]>,
    pub cryptocurrency: Option<&'a [T]>,
}

impl TastyTrade {
    pub async fn get_market_data<'a>(
        &self,
        request: MarketDataRequest<'a,&str>,
    ) -> TastyResult<Items<MarketData>> {
        self.get(format!("/market-data/by-type?index={}&equity={}&equity_option={}&future={}&future_option={}&cryptocurrency={}", 
        join_symbols_optional(request.index),
        join_symbols_optional(request.equity), 
        join_symbols_optional(request.equity_option),
        join_symbols_optional(request.future),
        join_symbols_optional(request.future_option), 
        join_symbols_optional(request.cryptocurrency),
    ))
            .await
    }
}