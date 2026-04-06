use std::collections::HashMap;
use std::env;
use std::time::Duration;
use tastytrade::dxfeed::{self, EventData};
use tastytrade::utils::config::TastyTradeConfig;
use tastytrade::{Symbol, TastyTrade};
use tokio::time::{Instant, timeout};

fn env_u64(key: &str, default: u64) -> u64 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_symbol_list(key: &str, default: &str) -> Vec<Symbol> {
    let raw = env::var(key).unwrap_or_else(|_| default.to_string());
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(Symbol::from)
        .collect()
}

fn merge_unique(base: &mut Vec<Symbol>, extras: &[Symbol]) {
    for sym in extras {
        if !base.iter().any(|existing| existing == sym) {
            base.push(sym.clone());
        }
    }
}

fn bump_counter(counters: &mut HashMap<String, usize>, symbol: &str) {
    *counters.entry(symbol.to_string()).or_insert(0) += 1;
}

#[tokio::main]
async fn main() {
    let config = TastyTradeConfig::from_env();

    if !config.has_valid_credentials() {
        eprintln!("Error: Missing TastyTrade credentials!");
        eprintln!("Please make sure you have:");
        eprintln!("1. Copied .env.example to .env: cp .env.example .env");
        eprintln!("2. Set TASTYTRADE_USERNAME and TASTYTRADE_PASSWORD in .env");
        eprintln!("3. Set TASTYTRADE_USE_DEMO=true for sandbox testing");
        std::process::exit(1);
    }

    let mut quote_symbols = env_symbol_list("QUOTE_STREAM_SYMBOLS", "SPY,AAPL");
    let mut trade_symbols = env_symbol_list("TRADE_STREAM_SYMBOLS", "SPY,AAPL");
    let extra_symbols = env_symbol_list("EXTRA_QUOTE_SYMBOLS", "MSFT");

    if quote_symbols.is_empty() {
        quote_symbols = vec![Symbol::from("SPY"), Symbol::from("AAPL")];
    }
    if trade_symbols.is_empty() {
        trade_symbols = quote_symbols.clone();
    }

    let max_quote_events = env_u64("MAX_QUOTE_EVENTS", 20) as usize;
    let max_trade_events = env_u64("MAX_TRADE_EVENTS", 12) as usize;
    let dynamic_add_after = env_u64("DYNAMIC_ADD_AFTER", 5) as usize;
    let stall_timeout_secs = env_u64("STALL_TIMEOUT_SECS", 10);
    let max_runtime_secs = env_u64("MAX_RUNTIME_SECS", 60);

    let greeks_symbol = env::var("GREEKS_SYMBOL")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    println!("Attempting to login with username: {}", config.client_id);
    println!("Using demo environment: {}", config.use_demo);
    println!(
        "Run config: quotes={:?}, trades={:?}, extra={:?}, max_quote_events={}, max_trade_events={}, stall_timeout={}s, max_runtime={}s",
        quote_symbols,
        trade_symbols,
        extra_symbols,
        max_quote_events,
        max_trade_events,
        stall_timeout_secs,
        max_runtime_secs
    );
    if let Some(sym) = &greeks_symbol {
        println!("Greeks test enabled with GREEKS_SYMBOL={sym}");
    } else {
        println!("Greeks test disabled (set GREEKS_SYMBOL to enable it)");
    }

    let tasty = match TastyTrade::login(&config).await {
        Ok(client) => {
            println!("✅ Login successful!");
            client
        }
        Err(e) => {
            eprintln!("❌ Login failed: {}", e);
            eprintln!("\nTroubleshooting:");
            eprintln!("1. Verify your credentials are correct");
            eprintln!("2. Make sure TASTYTRADE_USE_DEMO=true for sandbox");
            eprintln!("3. Check if your account has API access enabled");
            std::process::exit(1);
        }
    };

    let mut streamer = match tasty.create_quote_streamer().await {
        Ok(s) => {
            println!("✅ Quote streamer created successfully!");
            s
        }
        Err(e) => {
            eprintln!("❌ Failed to create quote streamer: {}", e);
            std::process::exit(1);
        }
    };

    let mut quote_sub = streamer.create_sub(dxfeed::DXF_ET_QUOTE);
    quote_sub.add_symbols(&quote_symbols);

    let mut trade_sub = streamer.create_sub(dxfeed::DXF_ET_TRADE);
    trade_sub.add_symbols(&trade_symbols);

    let mut greeks_sub = greeks_symbol.as_ref().map(|sym| {
        let mut sub = streamer.create_sub(dxfeed::DXF_ET_GREEKS);
        let symbols = [Symbol::from(sym.as_str())];
        sub.add_symbols(&symbols);
        sub
    });

    println!("📡 Streaming quotes + trades concurrently...");
    let start = Instant::now();
    let stall_timeout = Duration::from_secs(stall_timeout_secs);
    let max_runtime = Duration::from_secs(max_runtime_secs);

    let mut quote_count: usize = 0;
    let mut trade_count: usize = 0;
    let mut quote_by_symbol: HashMap<String, usize> = HashMap::new();
    let mut trade_by_symbol: HashMap<String, usize> = HashMap::new();
    let mut saw_unexpected_quote_payload = false;
    let mut saw_unexpected_trade_payload = false;
    let mut dynamic_symbols_added = false;

    while (quote_count < max_quote_events || trade_count < max_trade_events) && start.elapsed() < max_runtime {
        tokio::select! {
            quote_res = timeout(stall_timeout, quote_sub.get_event()), if quote_count < max_quote_events => {
                match quote_res {
                    Ok(Ok(ev)) => {
                        match &ev.data {
                            EventData::Quote(data) => {
                                quote_count += 1;
                                bump_counter(&mut quote_by_symbol, &ev.sym);
                                println!("[QUOTE {quote_count:>3}] {} bid=${:.2} ask=${:.2}", ev.sym, data.bid_price, data.ask_price);

                                // Dynamic symbol add validates command ordering + registration updates.
                                if !dynamic_symbols_added && quote_count >= dynamic_add_after && !extra_symbols.is_empty() {
                                    println!("➕ Adding extra symbols to quote+trade subs: {:?}", extra_symbols);
                                    quote_sub.add_symbols(&extra_symbols);
                                    trade_sub.add_symbols(&extra_symbols);
                                    merge_unique(&mut quote_symbols, &extra_symbols);
                                    merge_unique(&mut trade_symbols, &extra_symbols);
                                    dynamic_symbols_added = true;
                                }
                            }
                            other => {
                                if !saw_unexpected_quote_payload {
                                    println!("⚠️ Quote subscription received unexpected payload variant once: {other:?}");
                                    saw_unexpected_quote_payload = true;
                                }
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        eprintln!("❌ Quote subscription error: {e:?}");
                        break;
                    }
                    Err(_) => {
                        eprintln!("⏱️ No quote event within {}s (stall timeout)", stall_timeout_secs);
                        break;
                    }
                }
            }
            trade_res = timeout(stall_timeout, trade_sub.get_event()), if trade_count < max_trade_events => {
                match trade_res {
                    Ok(Ok(ev)) => {
                        match &ev.data {
                            EventData::Trade(data) => {
                                trade_count += 1;
                                bump_counter(&mut trade_by_symbol, &ev.sym);
                                println!("[TRADE {trade_count:>3}] {} px=${:.2} size={}", ev.sym, data.price, data.size);
                            }
                            other => {
                                if !saw_unexpected_trade_payload {
                                    println!("⚠️ Trade subscription received unexpected payload variant once: {other:?}");
                                    saw_unexpected_trade_payload = true;
                                }
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        eprintln!("❌ Trade subscription error: {e:?}");
                        break;
                    }
                    Err(_) => {
                        eprintln!("⏱️ No trade event within {}s (stall timeout)", stall_timeout_secs);
                        break;
                    }
                }
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f64().max(1.0);
    println!(
        "\nPhase 1 complete in {:.2}s | quote_events={} ({:.2}/s) | trade_events={} ({:.2}/s)",
        elapsed,
        quote_count,
        quote_count as f64 / elapsed,
        trade_count,
        trade_count as f64 / elapsed
    );
    println!("Quote events by symbol: {:?}", quote_by_symbol);
    println!("Trade events by symbol: {:?}", trade_by_symbol);

    println!("\n🧪 Lifecycle test: close trade sub and confirm quote sub still works");
    streamer.close_sub(trade_sub);
    match timeout(stall_timeout, quote_sub.get_event()).await {
        Ok(Ok(ev)) => println!("✅ Quote sub still active after trade sub close; next symbol={}", ev.sym),
        Ok(Err(e)) => eprintln!("⚠️ Quote sub error after trade sub close: {e:?}"),
        Err(_) => eprintln!("⚠️ No quote event after trade sub close within {}s", stall_timeout_secs),
    }

    if let Some(mut gsub) = greeks_sub.take() {
        println!("\n🧪 Optional greeks test with symbol={}", greeks_symbol.unwrap_or_default());
        match timeout(stall_timeout, gsub.get_event()).await {
            Ok(Ok(ev)) => match &ev.data {
                EventData::Greeks(data) => {
                    println!(
                        "✅ Greeks event {}: delta={:.4} gamma={:.4} theta={:.4} vega={:.4} rho={:.4}",
                        ev.sym, data.delta, data.gamma, data.theta, data.vega, data.rho
                    );
                }
                other => {
                    println!("⚠️ Greeks sub received non-greeks payload: {other:?}");
                }
            },
            Ok(Err(e)) => eprintln!("⚠️ Greeks subscription error: {e:?}"),
            Err(_) => eprintln!(
                "⚠️ No greeks event within {}s. This is expected unless GREEKS_SYMBOL is valid and active.",
                stall_timeout_secs
            ),
        }
        streamer.close_sub(gsub);
    }

    println!("\n🧪 Lifecycle test: close quote sub then create a fresh quote sub");
    streamer.close_sub(quote_sub);
    let mut resub_quote = streamer.create_sub(dxfeed::DXF_ET_QUOTE);
    let resub_symbols = if quote_symbols.is_empty() {
        vec![Symbol::from("SPY")]
    } else {
        vec![quote_symbols[0].clone()]
    };
    resub_quote.add_symbols(&resub_symbols);

    match timeout(stall_timeout, resub_quote.get_event()).await {
        Ok(Ok(ev)) => println!("✅ Re-subscribe worked; received quote for {}", ev.sym),
        Ok(Err(e)) => eprintln!("⚠️ Re-subscribe quote error: {e:?}"),
        Err(_) => eprintln!("⚠️ No event after re-subscribe within {}s", stall_timeout_secs),
    }
    streamer.close_sub(resub_quote);

    println!("\nShutting down quote streamer");
    streamer.shutdown().await;
    println!("Quote streamer shut down");
}
