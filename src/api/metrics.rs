use crate::api::base::Items;
use crate::utils::join::join_symbols;
use crate::{AsSymbol, TastyResult, TastyTrade, types::metrics::OptionMetrics};

impl TastyTrade {
    pub async fn get_option_metrics(
        &self,
        symbols: &[impl AsSymbol],
    ) -> TastyResult<Items<OptionMetrics>> {
        self.get(format!("/market-metrics?symbols={}", join_symbols(symbols)))
            .await
    }
}
