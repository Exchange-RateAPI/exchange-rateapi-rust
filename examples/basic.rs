//! Basic usage example for the Exchange Rate API Rust SDK.
//!
//! Run with:
//!   cargo run --example basic
//!
//! Make sure to set the EXCHANGE_RATE_API_KEY environment variable first:
//!   export EXCHANGE_RATE_API_KEY="era_live_your_api_key"

use exchange_rateapi::{ExchangeRateAPI, Period};

fn main() {
    // Read API key from environment
    let api_key = std::env::var("EXCHANGE_RATE_API_KEY")
        .expect("Set EXCHANGE_RATE_API_KEY environment variable");

    let client = ExchangeRateAPI::new(&api_key);

    // -----------------------------------------------------------------------
    // 1. Get latest rates
    // -----------------------------------------------------------------------
    println!("--- Latest Rates (USD) ---");
    match client.latest("USD", Some("EUR,GBP,JPY,CAD")) {
        Ok(resp) => {
            for (currency, rate) in &resp.rates {
                println!("  {} = {}", currency, rate);
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    // -----------------------------------------------------------------------
    // 2. Convert currency
    // -----------------------------------------------------------------------
    println!("\n--- Convert 100 USD to EUR ---");
    match client.convert("USD", "EUR", 100.0) {
        Ok(resp) => {
            println!("  {} {} = {} {}", resp.amount, resp.from, resp.result, resp.to);
            println!("  Rate: {}", resp.rate);
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    // -----------------------------------------------------------------------
    // 3. Get a single exchange rate
    // -----------------------------------------------------------------------
    println!("\n--- Single Rate: GBP/JPY ---");
    match client.get_rate("GBP", "JPY") {
        Ok(rate) => println!("  1 GBP = {} JPY", rate),
        Err(e) => eprintln!("Error: {}", e),
    }

    // -----------------------------------------------------------------------
    // 4. Historical rates for a specific date
    // -----------------------------------------------------------------------
    println!("\n--- Historical Rates (2025-01-15, EUR base) ---");
    match client.for_date("2025-01-15", "EUR", Some("USD,GBP")) {
        Ok(resp) => {
            println!("  Date: {}", resp.date);
            for (currency, rate) in &resp.rates {
                println!("  {} = {}", currency, rate);
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    // -----------------------------------------------------------------------
    // 5. Time series
    // -----------------------------------------------------------------------
    println!("\n--- Time Series (USD/EUR, Jan 2025) ---");
    match client.time_series("2025-01-01", "2025-01-07", "USD", Some("EUR")) {
        Ok(resp) => {
            let mut dates: Vec<&String> = resp.rates.keys().collect();
            dates.sort();
            for date in dates {
                if let Some(rate) = resp.rates[date].get("EUR") {
                    println!("  {}: EUR = {}", date, rate);
                }
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    // -----------------------------------------------------------------------
    // 6. List all symbols
    // -----------------------------------------------------------------------
    println!("\n--- Supported Currencies (first 10) ---");
    match client.symbols() {
        Ok(resp) => {
            let mut symbols: Vec<(&String, &String)> = resp.symbols.iter().collect();
            symbols.sort_by_key(|(code, _)| code.to_owned());
            for (code, name) in symbols.iter().take(10) {
                println!("  {}: {}", code, name);
            }
            println!("  ... and {} more", resp.symbols.len().saturating_sub(10));
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    // -----------------------------------------------------------------------
    // 7. Historical rates with preset period
    // -----------------------------------------------------------------------
    println!("\n--- Last 7 Days: USD/EUR ---");
    match client.get_historical_rates("USD", "EUR", Period::SevenDays) {
        Ok(resp) => {
            let mut dates: Vec<&String> = resp.rates.keys().collect();
            dates.sort();
            for date in dates {
                if let Some(rate) = resp.rates[date].get("EUR") {
                    println!("  {}: {}", date, rate);
                }
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
