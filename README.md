# exchange-rateapi

[![Crates.io](https://img.shields.io/crates/v/exchange-rateapi.svg)](https://crates.io/crates/exchange-rateapi)
[![Docs.rs](https://docs.rs/exchange-rateapi/badge.svg)](https://docs.rs/exchange-rateapi)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Official Rust SDK for [Exchange Rate API](https://exchange-rateapi.com) -- real-time mid-market exchange rates for 160+ currencies.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
exchange-rateapi = "1.0"
```

Or install via `cargo`:

```sh
cargo add exchange-rateapi
```

## Quick Start

```rust
use exchange_rateapi::ExchangeRateAPI;

fn main() {
    let client = ExchangeRateAPI::new("era_live_your_api_key");

    // Get latest USD rates
    let resp = client.latest("USD", None).unwrap();
    println!("USD/EUR: {}", resp.rates["EUR"]);

    // Convert 100 USD to GBP
    let result = client.convert("USD", "GBP", 100.0).unwrap();
    println!("100 USD = {} GBP", result.result);
}
```

## API Key

Sign up at [exchange-rateapi.com](https://exchange-rateapi.com) to get your API key. Keys use the format `era_live_...`.

Authentication is handled automatically via Bearer token in the Authorization header.

## Methods

### `latest(base, symbols)` -- Get Latest Rates

Fetch the most recent exchange rates for a base currency.

```rust
// All currencies
let resp = client.latest("USD", None).unwrap();

// Specific currencies only
let resp = client.latest("USD", Some("EUR,GBP,JPY")).unwrap();

println!("Base: {}", resp.base);
println!("Date: {}", resp.date);
for (code, rate) in &resp.rates {
    println!("{}: {}", code, rate);
}
```

### `convert(from, to, amount)` -- Convert Currency

Convert an amount between two currencies.

```rust
let resp = client.convert("USD", "EUR", 250.0).unwrap();

println!("{} {} = {} {}", resp.amount, resp.from, resp.result, resp.to);
println!("Rate: {}", resp.rate);
```

### `for_date(date, base, symbols)` -- Historical Rates

Get exchange rates for a specific historical date.

```rust
let resp = client.for_date("2025-01-15", "EUR", Some("USD,GBP")).unwrap();

println!("Date: {}", resp.date);
for (code, rate) in &resp.rates {
    println!("{}: {}", code, rate);
}
```

### `time_series(start, end, base, symbols)` -- Time Series

Fetch rates over a date range.

```rust
let resp = client.time_series(
    "2025-01-01",
    "2025-01-31",
    "USD",
    Some("EUR,GBP"),
).unwrap();

for (date, rates) in &resp.rates {
    println!("{}: EUR={}, GBP={}", date, rates["EUR"], rates["GBP"]);
}
```

### `symbols()` -- List All Currencies

Get all supported currency codes and their names.

```rust
let resp = client.symbols().unwrap();

for (code, name) in &resp.symbols {
    println!("{}: {}", code, name);
}
```

### `get_rate(from, to)` -- Single Pair Rate

Convenience method to get a single exchange rate as an `f64`.

```rust
let rate = client.get_rate("USD", "EUR").unwrap();
println!("1 USD = {} EUR", rate);
```

### `get_historical_rates(source, target, period)` -- Preset Period Rates

Fetch rates for a preset time period relative to today.

```rust
use exchange_rateapi::Period;

// Available periods: OneDay, SevenDays, ThirtyDays, OneYear
let resp = client.get_historical_rates("USD", "EUR", Period::ThirtyDays).unwrap();

for (date, rates) in &resp.rates {
    println!("{}: {}", date, rates["EUR"]);
}

// You can also parse period strings
let period = Period::from_str("7d").unwrap(); // "1d", "7d", "30d", "1y"
```

## Error Handling

All methods return `Result<T, ExchangeRateAPIError>`. The error enum has three variants:

```rust
use exchange_rateapi::{ExchangeRateAPI, ExchangeRateAPIError};

let client = ExchangeRateAPI::new("era_live_your_key");

match client.latest("USD", None) {
    Ok(resp) => println!("Got {} rates", resp.rates.len()),
    Err(ExchangeRateAPIError::HttpError(e)) => {
        eprintln!("Network error: {}", e);
    }
    Err(ExchangeRateAPIError::ApiError { status, message }) => {
        eprintln!("API returned {}: {}", status, message);
    }
    Err(ExchangeRateAPIError::ParseError(msg)) => {
        eprintln!("Could not parse response: {}", msg);
    }
}
```

## Response Types

| Method | Return Type |
|---|---|
| `latest` | `LatestResponse` |
| `convert` | `ConvertResponse` |
| `for_date` | `HistoricalResponse` |
| `time_series` | `TimeSeriesResponse` |
| `symbols` | `SymbolsResponse` |
| `get_rate` | `f64` |
| `get_historical_rates` | `TimeSeriesResponse` |

All response structs implement `Debug`, `Clone`, `Serialize`, and `Deserialize`.

## Requirements

- Rust 2021 edition or later
- An API key from [exchange-rateapi.com](https://exchange-rateapi.com)

## Links

- [API Documentation](https://exchange-rateapi.com/docs)
- [Crate Documentation](https://docs.rs/exchange-rateapi)
- [GitHub Repository](https://github.com/Exchange-RateAPI/exchange-rateapi-rust)

## License

MIT License. See [LICENSE](LICENSE) for details.
