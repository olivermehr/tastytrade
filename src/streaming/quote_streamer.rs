// For quote_streamer.rs
use crate::TastyTrade;
use crate::types::dxfeed;
use crate::{AsSymbol, Symbol, TastyResult, TastyTradeError};
use dxlink::{DXLinkClient, EventType, FeedSubscription, MarketEvent};
use pretty_simple_display::{DebugPretty, DisplaySimple};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, atomic};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

#[derive(DebugPretty, DisplaySimple, Serialize, PartialEq, Eq, Hash, Clone, Copy)]
pub struct SubscriptionId(usize);

pub struct QuoteSubscription {
    pub id: SubscriptionId,
    channel_id: u32,
    command_sender: mpsc::Sender<DXLinkCommand>,
    event_types: i32, // Keep for compatibility with existing code
    dxlink_receiver: mpsc::Receiver<MarketEvent>, // New DXLink event receiver
    dxlink_tx: mpsc::Sender<MarketEvent>, // To send events to DXLink
    symbols: Vec<Symbol>, // To track subscribed symbols
}

impl QuoteSubscription {
    /// Add symbols to subscription. See the "Note on symbology" section in [`QuoteSubscription`]
    pub fn add_symbols<S: AsSymbol>(&mut self, symbols: &[S]) {
        let symbols: Vec<Symbol> = symbols.iter().map(|sym| sym.as_symbol()).collect();
        let cloned_symbols = symbols.clone();
        let cloned_symbols_2 = symbols.clone();

        // Prepare subscription requests for DXLink
        let subscriptions = symbols
            .into_iter()
            .flat_map(|sym| {
                // Transform dxfeed flags to DXLink event types
                let event_flags = self.event_types;
                let mut subscriptions = Vec::new();

                if (event_flags & dxfeed::DXF_ET_QUOTE) != 0 {
                    subscriptions.push(FeedSubscription {
                        event_type: "Quote".to_string(),
                        symbol: sym.0.clone(),
                        from_time: None,
                        source: None,
                    });
                }
                if (event_flags & dxfeed::DXF_ET_TRADE) != 0 {
                    subscriptions.push(FeedSubscription {
                        event_type: "Trade".to_string(),
                        symbol: sym.0.clone(),
                        from_time: None,
                        source: None,
                    });
                }
                if (event_flags & dxfeed::DXF_ET_GREEKS) != 0 {
                    subscriptions.push(FeedSubscription {
                        event_type: "Greeks".to_string(),
                        symbol: sym.0.clone(),
                        from_time: None,
                        source: None,
                    });
                }
                subscriptions
            })
            .collect::<Vec<FeedSubscription>>();
        let cloned_command_sender = self.command_sender.clone();
        let cloned_dxlink_tx = self.dxlink_tx.clone();
        let channel_id = self.channel_id;
        let subscription_id = self.id;
        self.symbols.extend(cloned_symbols);

        tokio::spawn(async move {
            if let Err(e) = cloned_command_sender
                .send(DXLinkCommand::AddEventSender(
                    subscription_id,
                    cloned_symbols_2,
                    cloned_dxlink_tx,
                ))
                .await
            {
                error!("Failed to add event sender: {}", e);
            }
            if !subscriptions.is_empty()
                && let Err(e) = cloned_command_sender
                    .send(DXLinkCommand::Subscribe(channel_id, subscriptions))
                    .await
            {
                error!("Failed to send subscription command: {}", e);
            }
        });
    }

    /// Receive one event from feed. Yields if there are no events.
    /// Compatible with previous interface
    pub async fn get_event(&mut self) -> Result<dxfeed::Event, TastyTradeError> {
        // Try to receive event from DXLink
        match self.dxlink_receiver.recv().await {
            Some(market_event) => {
                // Convert from DXLink MarketEvent to dxfeed Event
                match market_event {
                    MarketEvent::Quote(quote) => {
                        let symbol = quote.event_symbol;
                        let data = dxfeed::EventData::Quote(dxfeed::DxfQuoteT {
                            time: 0,
                            sequence: 0,
                            time_nanos: 0,
                            bid_time: 0,
                            bid_exchange_code: 0,
                            bid_price: quote.bid_price,
                            ask_price: quote.ask_price,
                            bid_size: quote.bid_size as i64,
                            ask_time: 0,
                            ask_size: quote.ask_size as i64,
                            ask_exchange_code: 0,
                            scope: 0,
                        });
                        Ok(dxfeed::Event { sym: symbol, data })
                    }
                    MarketEvent::Trade(trade) => {
                        // Convert Trade to dxfeed format
                        let symbol = trade.event_symbol;
                        let data = dxfeed::EventData::Trade(dxfeed::DxfTradeT {
                            time: 0,
                            sequence: 0,
                            time_nanos: 0,
                            exchange_code: 0,
                            price: trade.price,
                            size: trade.size as i64,

                            tick: 0,
                            change: 0.0,
                            day_id: 0,
                            day_volume: 0.0,
                            day_turnover: 0.0,
                            raw_flags: 0,
                            direction: 0,
                            is_eth: 0,
                            scope: 0,
                        });
                        Ok(dxfeed::Event { sym: symbol, data })
                    }
                    MarketEvent::Greeks(greeks) => {
                        // Convert Greeks to dxfeed format
                        let symbol = greeks.event_symbol;
                        let data = dxfeed::EventData::Greeks(dxfeed::DxfGreeksT {
                            event_flags: 0,
                            index: 0,
                            time: 0,
                            price: 0.0,
                            volatility: 0.0,
                            delta: greeks.delta,
                            gamma: greeks.gamma,
                            theta: greeks.theta,
                            vega: greeks.vega,
                            rho: greeks.rho,
                        });
                        Ok(dxfeed::Event { sym: symbol, data })
                    }
                }
            }
            None => {
                debug!("Channel is closed");
                Err(TastyTradeError::Streaming("Channel is closed".to_string()))
            }
        }
    }
}

// Commands for DXLink client to execute
enum DXLinkCommand {
    Subscribe(u32, Vec<FeedSubscription>),
    Unsubscribe(u32, Vec<FeedSubscription>),
    AddEventSender(SubscriptionId, Vec<Symbol>, mpsc::Sender<MarketEvent>),
    RemoveEventSender(SubscriptionId, Vec<Symbol>),
    Disconnect,
}

pub struct QuoteStreamer {
    #[allow(dead_code)]
    channel_id: Option<u32>,
    next_sub_id: Arc<atomic::AtomicUsize>,
    dxlink_command_tx: Option<mpsc::Sender<DXLinkCommand>>,
}

impl QuoteStreamer {
    pub async fn connect(tasty: &TastyTrade) -> TastyResult<Self> {
        let tokens = tasty.quote_streamer_tokens().await?;
        debug!("Obtained tokens for DXLink: {}", tokens.token);

        // Create DXLink client
        let mut client = DXLinkClient::new(&tokens.streamer_url, &tokens.token);

        // Connect to server
        info!("Connecting to DXLink server: {}", tokens.streamer_url);
        let mut market_data_receiver = client.connect().await.map_err(|e| {
            TastyTradeError::Streaming(format!("Error connecting to DXLink: {}", e))
        })?;

        // Create channel for market data
        let channel_id = match client.create_feed_channel("AUTO").await {
            Ok(id) => id,
            Err(e) => {
                return Err(TastyTradeError::Streaming(format!(
                    "Error creating DXLink channel: {}",
                    e
                )));
            }
        };
        info!("DXLink channel created: {}", channel_id);

        // Configure feed for different event types
        if let Err(e) = client
            .setup_feed(
                channel_id,
                &[EventType::Quote, EventType::Trade, EventType::Greeks],
            )
            .await
        {
            return Err(TastyTradeError::Streaming(format!(
                "Error configuring DXLink feed: {}",
                e
            )));
        }

        // Create event senders map within a arc mutex
        let event_senders_map = Arc::new(Mutex::new(HashMap::<
            SubscriptionId,
            mpsc::Sender<MarketEvent>,
        >::new()));
        let subscription_id_map =
            Arc::new(Mutex::new(HashMap::<Symbol, Vec<SubscriptionId>>::new()));

        // Create command channel
        let (command_tx, mut command_rx) = mpsc::channel::<DXLinkCommand>(100);

        let event_senders_clone_1 = event_senders_map.clone();
        let subscription_id_map_clone_1 = subscription_id_map.clone();

        // Move rx directly into the spawned task
        tokio::spawn(async move {
            // Use rx directly, don't try to borrow from event_stream
            while let Some(event) = market_data_receiver.recv().await {
                // Determine which symbol this event is for
                let symbol = match &event {
                    MarketEvent::Quote(quote) => Symbol::from(&quote.event_symbol),
                    MarketEvent::Trade(trade) => Symbol::from(&trade.event_symbol),
                    MarketEvent::Greeks(greeks) => Symbol::from(&greeks.event_symbol),
                };

                if let Ok(subscription_id_guard) = subscription_id_map_clone_1.lock()
                    && let Some(subscription_id) = subscription_id_guard.get(&symbol)
                {
                    for subscription_id in subscription_id {
                        if let Ok(event_senders_guard) = event_senders_clone_1.lock()
                            && let Some(sender) = event_senders_guard.get(subscription_id)
                        {
                            let _ = sender.try_send(event.clone());
                        }
                    }
                }
            }
        });

        let event_senders_clone_2 = event_senders_map.clone();
        let subscription_id_map_clone_2 = subscription_id_map.clone();
        // Spawn task to handle DXLink commands
        tokio::spawn(async move {
            while let Some(cmd) = command_rx.recv().await {
                match cmd {
                    DXLinkCommand::Subscribe(channel_id, subscriptions) => {
                        info!("Subscribing to symbols: {:?}", subscriptions);
                        if let Err(e) = client.subscribe(channel_id, subscriptions).await {
                            error!("Error subscribing to symbols: {}", e);
                        }
                    }
                    DXLinkCommand::Unsubscribe(channel_id, subscriptions) => {
                        if let Err(e) = client.unsubscribe(channel_id, subscriptions).await {
                            error!("Error unsubscribing from symbols: {}", e);
                        }
                    }
                    DXLinkCommand::Disconnect => {
                        if let Err(e) = client.disconnect().await {
                            warn!("Error disconnecting from DXLink: {}", e);
                        }
                        break; // Exit the loop after disconnecting
                    }
                    DXLinkCommand::AddEventSender(subscription_id, symbols_vec, sender) => {
                        if let Ok(mut subscription_id_guard) = subscription_id_map_clone_2.lock() {
                            for symbol in symbols_vec.into_iter() {
                                debug!(
                                    "Add event sender for subscription {} to symbol {}",
                                    subscription_id, symbol
                                );
                                subscription_id_guard
                                    .entry(symbol)
                                    .or_default()
                                    .push(subscription_id);
                            }
                        }
                        if let Ok(mut event_senders_guard) = event_senders_clone_2.lock() {
                            event_senders_guard.insert(subscription_id, sender);
                            debug!("Added event sender for subscription {}", subscription_id);
                        }
                    }
                    DXLinkCommand::RemoveEventSender(subscription_id, symbols_vec) => {
                        if let Ok(mut subscription_id_guard) = subscription_id_map_clone_2.lock() {
                            for symbol in symbols_vec.into_iter() {
                                subscription_id_guard
                                    .get_mut(&symbol)
                                    .unwrap()
                                    .retain(|id| *id != subscription_id);
                                debug!("Removed subscription id for symbol {}", symbol);
                            }
                        }

                        if let Ok(mut event_senders_guard) = event_senders_clone_2.lock() {
                            event_senders_guard.remove(&subscription_id);
                            debug!("Removed event sender for subscription {}", subscription_id);
                        }
                    }
                }
            }
            debug!("DXLink command handler terminated");
        });

        Ok(Self {
            channel_id: Some(channel_id),
            next_sub_id: Arc::new(atomic::AtomicUsize::new(0)),
            dxlink_command_tx: Some(command_tx),
        })
    }

    /// Create a subscription to market data. See `dxfeed::DXF_ET_*` for possible event types.
    pub fn create_sub(&mut self, flags: i32) -> QuoteSubscription {
        let id = SubscriptionId(self.next_sub_id.fetch_add(1, Ordering::Relaxed));

        // Set up channels for events
        let (dxlink_tx, dxlink_rx) = mpsc::channel(100);

        // Create subscription
        QuoteSubscription {
            id,
            channel_id: self.channel_id.unwrap(),
            command_sender: self.dxlink_command_tx.clone().unwrap(),
            event_types: flags,
            dxlink_receiver: dxlink_rx,
            dxlink_tx,
            symbols: Vec::new(),
        }
    }

    /// Close and remove subscription by id.
    pub fn close_sub(&mut self, subscription: QuoteSubscription) {
        // Prepare unsubscribe requests
        let unsubscribe_requests = subscription
            .symbols
            .iter()
            .flat_map(|sym| {
                let mut requests = Vec::new();
                let event_flags = subscription.event_types;

                if (event_flags & dxfeed::DXF_ET_QUOTE) != 0 {
                    requests.push(FeedSubscription {
                        event_type: "Quote".to_string(),
                        symbol: sym.0.clone(),
                        from_time: None,
                        source: None,
                    });
                }

                if (event_flags & dxfeed::DXF_ET_TRADE) != 0 {
                    requests.push(FeedSubscription {
                        event_type: "Trade".to_string(),
                        symbol: sym.0.clone(),
                        from_time: None,
                        source: None,
                    });
                }

                if (event_flags & dxfeed::DXF_ET_GREEKS) != 0 {
                    requests.push(FeedSubscription {
                        event_type: "Greeks".to_string(),
                        symbol: sym.0.clone(),
                        from_time: None,
                        source: None,
                    });
                }

                requests
            })
            .collect::<Vec<FeedSubscription>>();

        // Execute unsubscribe via command channel
        if let (Some(tx), Some(channel_id)) = (&self.dxlink_command_tx, self.channel_id) {
            let tx_clone = tx.clone();
            let channel = channel_id;

            tokio::spawn(async move {
                // Unregister the event sender
                if let Err(e) = tx_clone
                    .send(DXLinkCommand::RemoveEventSender(
                        subscription.id,
                        subscription.symbols,
                    ))
                    .await
                {
                    error!("Error unregistering event sender: {}", e);
                }

                // Unsubscribe from symbols
                if !unsubscribe_requests.is_empty()
                    && let Err(e) = tx_clone
                        .send(DXLinkCommand::Unsubscribe(channel, unsubscribe_requests))
                        .await
                {
                    error!("Error sending unsubscribe command: {}", e);
                }
            });
        }
    }

    pub async fn shutdown(&mut self) {
        // Send disconnect command to DXLink client
        if let Some(tx) = &self.dxlink_command_tx
            && let Err(e) = tx.send(DXLinkCommand::Disconnect).await
        {
            warn!("Error sending disconnect command: {}", e);
        }
    }
}

impl Drop for QuoteStreamer {
    fn drop(&mut self) {
        // Send disconnect command to DXLink client
        if let Some(tx) = &self.dxlink_command_tx
            && let Err(e) = tx.try_send(DXLinkCommand::Disconnect)
        {
            warn!("Error sending disconnect command: {}", e);
        }
    }
}
