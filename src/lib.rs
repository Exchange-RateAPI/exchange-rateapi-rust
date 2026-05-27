//! # Exchange Rate API SDK
//!
//! Official Rust SDK for [Exchange Rate API](https://exchange-rateapi.com) --
//! real-time mid-market exchange rates for 160+ currencies.
//!
//! ## Quick Start
//!
//! ```no_run
//! use exchange_rateapi::ExchangeRateAPI;
//!
//! let client = ExchangeRateAPI::new("era_live_your_api_key");
//!
//! // Get latest rates for USD
//! let response = client.latest("USD", None).unwrap();
//! println!("USD to EUR: {}", response.rates["EUR"]);
//!
//! // Convert 100 USD to GBP
//! let result = client.convert("USD", "GBP", 100.0).unwrap();
//! println!("100 USD = {} GBP", result.result);
//! ```

use std::collections::HashMap;
use std::fmt;

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::{Deserialize, Serialize};

/// Base URL for the Exchange Rate API.
const BASE_URL: &str = "https://exchange-rateapi.com";

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors returned by the Exchange Rate API SDK.
#[derive(Debug)]
pub enum ExchangeRateAPIError {
    /// An HTTP-level error from the underlying reqwest client.
    HttpError(reqwest::Error),
    /// An error returned by the Exchange Rate API itself (non-2xx response).
    ApiError {
        /// HTTP status code.
        status: u16,
        /// Human-readable error message from the API.
        message: String,
    },
    /// Failed to parse the API response body.
    ParseError(String),
}

impl fmt::Display for ExchangeRateAPIError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExchangeRateAPIError::HttpError(e) => write!(f, "HTTP error: {}", e),
            ExchangeRateAPIError::ApiError { status, message } => {
                write!(f, "API error ({}): {}", status, message)
            }
            ExchangeRateAPIError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for ExchangeRateAPIError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExchangeRateAPIError::HttpError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for ExchangeRateAPIError {
    fn from(err: reqwest::Error) -> Self {
        ExchangeRateAPIError::HttpError(err)
    }
}

impl From<serde_json::Error> for ExchangeRateAPIError {
    fn from(err: serde_json::Error) -> Self {
        ExchangeRateAPIError::ParseError(err.to_string())
    }
}

/// Convenience alias for results returned by this crate.
pub type Result<T> = std::result::Result<T, ExchangeRateAPIError>;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response from the `/v1/latest` endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LatestResponse {
    /// Whether the request was successful.
    pub success: bool,
    /// The base currency code (e.g. "USD").
    pub base: String,
    /// ISO-8601 date of the rates.
    pub date: String,
    /// Map of currency code to exchange rate.
    pub rates: HashMap<String, f64>,
}

/// Response from the `/v1/convert` endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConvertResponse {
    /// Whether the request was successful.
    pub success: bool,
    /// Source currency code.
    pub from: String,
    /// Target currency code.
    pub to: String,
    /// Amount that was converted.
    pub amount: f64,
    /// Converted result.
    pub result: f64,
    /// The exchange rate applied.
    pub rate: f64,
}

/// Response from the `/v1/history` endpoint (single date).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistoricalResponse {
    /// Whether the request was successful.
    pub success: bool,
    /// The base currency code.
    pub base: String,
    /// The date of the historical rates.
    pub date: String,
    /// Map of currency code to exchange rate.
    pub rates: HashMap<String, f64>,
}

/// Response from the `/v1/timeseries` endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeSeriesResponse {
    /// Whether the request was successful.
    pub success: bool,
    /// The base currency code.
    pub base: String,
    /// Start date of the series (inclusive).
    pub start_date: String,
    /// End date of the series (inclusive).
    pub end_date: String,
    /// Map of date string to currency-rate map.
    pub rates: HashMap<String, HashMap<String, f64>>,
}

/// A single currency entry returned by the `/v1/symbols` endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SymbolsResponse {
    /// Whether the request was successful.
    pub success: bool,
    /// Map of currency code to currency name.
    pub symbols: HashMap<String, String>,
}

/// Response from the `/v1/latest` endpoint when requesting a single pair.
/// Re-uses [`LatestResponse`] internally.
pub type SingleRateResponse = LatestResponse;

/// An API error body returned by the server.
#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

// ---------------------------------------------------------------------------
// Preset period helper
// ---------------------------------------------------------------------------

/// Preset time periods for [`ExchangeRateAPI::get_historical_rates`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    /// Last 1 day.
    OneDay,
    /// Last 7 days.
    SevenDays,
    /// Last 30 days.
    ThirtyDays,
    /// Last 365 days (1 year).
    OneYear,
}

impl Period {
    /// Returns the number of days this period represents.
    fn days(self) -> i64 {
        match self {
            Period::OneDay => 1,
            Period::SevenDays => 7,
            Period::ThirtyDays => 30,
            Period::OneYear => 365,
        }
    }

    /// Parses a short string tag into a `Period`.
    ///
    /// Accepted values: `"1d"`, `"7d"`, `"30d"`, `"1y"`.
    pub fn from_str(s: &str) -> Option<Period> {
        match s {
            "1d" => Some(Period::OneDay),
            "7d" => Some(Period::SevenDays),
            "30d" => Some(Period::ThirtyDays),
            "1y" => Some(Period::OneYear),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Simple date helpers (avoids pulling in chrono)
// ---------------------------------------------------------------------------

/// A minimal date representation (year, month, day) used for period calculations.
struct SimpleDate {
    year: i32,
    month: u32,
    day: u32,
}

impl SimpleDate {
    fn today() -> Self {
        // We use the system time to derive the current UTC date.
        let dur = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before UNIX epoch");
        let total_days = (dur.as_secs() / 86400) as i64;
        Self::from_epoch_days(total_days)
    }

    fn from_epoch_days(mut days: i64) -> Self {
        // Algorithm from Howard Hinnant's civil_from_days.
        days += 719_468;
        let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
        let doe = (days - era * 146_097) as u32; // day of era [0, 146096]
        let yoe =
            (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
        let y = (yoe as i64 + era * 400) as i32;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if m <= 2 { y + 1 } else { y };
        SimpleDate {
            year: y,
            month: m,
            day: d,
        }
    }

    fn subtract_days(&self, n: i64) -> Self {
        let epoch = self.to_epoch_days() - n;
        Self::from_epoch_days(epoch)
    }

    fn to_epoch_days(&self) -> i64 {
        let y = if self.month <= 2 {
            self.year as i64 - 1
        } else {
            self.year as i64
        };
        let m = if self.month <= 2 {
            self.month as i64 + 9
        } else {
            self.month as i64 - 3
        };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = (y - era * 400) as u64;
        let doy = (153 * (m as u64) + 2) / 5 + self.day as u64 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe as i64 - 719_468
    }

    fn format(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Client for the Exchange Rate API.
///
/// Create an instance with [`ExchangeRateAPI::new`] and call methods to
/// interact with the API endpoints.
///
/// # Example
///
/// ```no_run
/// use exchange_rateapi::ExchangeRateAPI;
///
/// let client = ExchangeRateAPI::new("era_live_your_api_key");
/// let rates = client.latest("USD", None).unwrap();
/// println!("{:?}", rates.rates);
/// ```
pub struct ExchangeRateAPI {
    client: Client,
    api_key: String,
}

impl ExchangeRateAPI {
    /// Creates a new `ExchangeRateAPI` client.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Your API key (format: `era_live_...`). Obtain one at
    ///   <https://exchange-rateapi.com>.
    pub fn new(api_key: &str) -> Self {
        let client = Client::new();
        ExchangeRateAPI {
            client,
            api_key: api_key.to_string(),
        }
    }

    // -- internal helpers ---------------------------------------------------

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let value = format!("Bearer {}", self.api_key);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&value).expect("invalid API key characters"),
        );
        headers
    }

    fn get(&self, path: &str, params: &[(&str, &str)]) -> Result<String> {
        let url = format!("{}{}", BASE_URL, path);
        let resp = self
            .client
            .get(&url)
            .headers(self.headers())
            .query(params)
            .send()?;

        let status = resp.status();
        let body = resp.text()?;

        if !status.is_success() {
            let message = match serde_json::from_str::<ApiErrorBody>(&body) {
                Ok(err_body) => err_body
                    .message
                    .or(err_body.error)
                    .unwrap_or_else(|| body.clone()),
                Err(_) => body.clone(),
            };
            return Err(ExchangeRateAPIError::ApiError {
                status: status.as_u16(),
                message,
            });
        }

        Ok(body)
    }

    // -- public endpoints ---------------------------------------------------

    /// Fetches the latest exchange rates for a base currency.
    ///
    /// # Arguments
    ///
    /// * `base` - The base currency code (e.g. `"USD"`).
    /// * `symbols` - Optional comma-separated list of target currencies
    ///   (e.g. `Some("EUR,GBP,JPY")`). Pass `None` to get all available
    ///   currencies.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use exchange_rateapi::ExchangeRateAPI;
    /// let client = ExchangeRateAPI::new("era_live_xxx");
    /// let resp = client.latest("USD", Some("EUR,GBP")).unwrap();
    /// println!("EUR rate: {}", resp.rates["EUR"]);
    /// ```
    pub fn latest(&self, base: &str, symbols: Option<&str>) -> Result<LatestResponse> {
        let mut params: Vec<(&str, &str)> = vec![("base", base)];
        if let Some(s) = symbols {
            params.push(("symbols", s));
        }
        let body = self.get("/v1/latest", &params)?;
        let parsed: LatestResponse = serde_json::from_str(&body)?;
        Ok(parsed)
    }

    /// Converts an amount from one currency to another.
    ///
    /// # Arguments
    ///
    /// * `from` - Source currency code (e.g. `"USD"`).
    /// * `to` - Target currency code (e.g. `"EUR"`).
    /// * `amount` - The amount to convert.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use exchange_rateapi::ExchangeRateAPI;
    /// let client = ExchangeRateAPI::new("era_live_xxx");
    /// let resp = client.convert("USD", "EUR", 250.0).unwrap();
    /// println!("250 USD = {} EUR", resp.result);
    /// ```
    pub fn convert(&self, from: &str, to: &str, amount: f64) -> Result<ConvertResponse> {
        let amount_str = amount.to_string();
        let params = [("from", from), ("to", to), ("amount", &amount_str)];
        let body = self.get("/v1/convert", &params)?;
        let parsed: ConvertResponse = serde_json::from_str(&body)?;
        Ok(parsed)
    }

    /// Fetches historical exchange rates for a specific date.
    ///
    /// # Arguments
    ///
    /// * `date` - The date in `YYYY-MM-DD` format.
    /// * `base` - The base currency code (e.g. `"USD"`).
    /// * `symbols` - Optional comma-separated list of target currencies.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use exchange_rateapi::ExchangeRateAPI;
    /// let client = ExchangeRateAPI::new("era_live_xxx");
    /// let resp = client.for_date("2025-01-15", "USD", Some("EUR,GBP")).unwrap();
    /// println!("Historical EUR rate: {}", resp.rates["EUR"]);
    /// ```
    pub fn for_date(
        &self,
        date: &str,
        base: &str,
        symbols: Option<&str>,
    ) -> Result<HistoricalResponse> {
        let mut params: Vec<(&str, &str)> = vec![("base", base)];
        if let Some(s) = symbols {
            params.push(("symbols", s));
        }
        let path = format!("/v1/history/{}", date);
        let body = self.get(&path, &params)?;
        let parsed: HistoricalResponse = serde_json::from_str(&body)?;
        Ok(parsed)
    }

    /// Fetches exchange rates over a date range (time series).
    ///
    /// # Arguments
    ///
    /// * `start` - Start date in `YYYY-MM-DD` format (inclusive).
    /// * `end` - End date in `YYYY-MM-DD` format (inclusive).
    /// * `base` - The base currency code.
    /// * `symbols` - Optional comma-separated list of target currencies.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use exchange_rateapi::ExchangeRateAPI;
    /// let client = ExchangeRateAPI::new("era_live_xxx");
    /// let resp = client.time_series("2025-01-01", "2025-01-31", "USD", Some("EUR")).unwrap();
    /// for (date, rates) in &resp.rates {
    ///     println!("{}: EUR = {}", date, rates["EUR"]);
    /// }
    /// ```
    pub fn time_series(
        &self,
        start: &str,
        end: &str,
        base: &str,
        symbols: Option<&str>,
    ) -> Result<TimeSeriesResponse> {
        let mut params: Vec<(&str, &str)> =
            vec![("start_date", start), ("end_date", end), ("base", base)];
        if let Some(s) = symbols {
            params.push(("symbols", s));
        }
        let body = self.get("/v1/timeseries", &params)?;
        let parsed: TimeSeriesResponse = serde_json::from_str(&body)?;
        Ok(parsed)
    }

    /// Lists all supported currency symbols.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use exchange_rateapi::ExchangeRateAPI;
    /// let client = ExchangeRateAPI::new("era_live_xxx");
    /// let resp = client.symbols().unwrap();
    /// for (code, name) in &resp.symbols {
    ///     println!("{}: {}", code, name);
    /// }
    /// ```
    pub fn symbols(&self) -> Result<SymbolsResponse> {
        let body = self.get("/v1/symbols", &[])?;
        let parsed: SymbolsResponse = serde_json::from_str(&body)?;
        Ok(parsed)
    }

    /// Gets the exchange rate for a single currency pair.
    ///
    /// This is a convenience wrapper around [`latest`](Self::latest) that
    /// returns just the rate as an `f64`.
    ///
    /// # Arguments
    ///
    /// * `from` - Source currency code (e.g. `"USD"`).
    /// * `to` - Target currency code (e.g. `"EUR"`).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use exchange_rateapi::ExchangeRateAPI;
    /// let client = ExchangeRateAPI::new("era_live_xxx");
    /// let rate = client.get_rate("USD", "EUR").unwrap();
    /// println!("1 USD = {} EUR", rate);
    /// ```
    pub fn get_rate(&self, from: &str, to: &str) -> Result<f64> {
        let resp = self.latest(from, Some(to))?;
        resp.rates.get(to).copied().ok_or_else(|| {
            ExchangeRateAPIError::ParseError(format!(
                "currency '{}' not found in response",
                to
            ))
        })
    }

    /// Gets historical rates for a preset time period.
    ///
    /// This is a convenience method that calculates the appropriate start and
    /// end dates and calls [`time_series`](Self::time_series).
    ///
    /// # Arguments
    ///
    /// * `source` - Base currency code (e.g. `"USD"`).
    /// * `target` - Target currency code (e.g. `"EUR"`).
    /// * `period` - One of the preset [`Period`] values.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use exchange_rateapi::{ExchangeRateAPI, Period};
    /// let client = ExchangeRateAPI::new("era_live_xxx");
    /// let resp = client.get_historical_rates("USD", "EUR", Period::SevenDays).unwrap();
    /// for (date, rates) in &resp.rates {
    ///     println!("{}: {}", date, rates["EUR"]);
    /// }
    /// ```
    pub fn get_historical_rates(
        &self,
        source: &str,
        target: &str,
        period: Period,
    ) -> Result<TimeSeriesResponse> {
        let today = SimpleDate::today();
        let start = today.subtract_days(period.days());
        self.time_series(&start.format(), &today.format(), source, Some(target))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_period_from_str() {
        assert_eq!(Period::from_str("1d"), Some(Period::OneDay));
        assert_eq!(Period::from_str("7d"), Some(Period::SevenDays));
        assert_eq!(Period::from_str("30d"), Some(Period::ThirtyDays));
        assert_eq!(Period::from_str("1y"), Some(Period::OneYear));
        assert_eq!(Period::from_str("invalid"), None);
    }

    #[test]
    fn test_period_days() {
        assert_eq!(Period::OneDay.days(), 1);
        assert_eq!(Period::SevenDays.days(), 7);
        assert_eq!(Period::ThirtyDays.days(), 30);
        assert_eq!(Period::OneYear.days(), 365);
    }

    #[test]
    fn test_simple_date_format() {
        let d = SimpleDate {
            year: 2025,
            month: 3,
            day: 5,
        };
        assert_eq!(d.format(), "2025-03-05");
    }

    #[test]
    fn test_simple_date_roundtrip() {
        let d = SimpleDate {
            year: 2025,
            month: 6,
            day: 15,
        };
        let epoch = d.to_epoch_days();
        let d2 = SimpleDate::from_epoch_days(epoch);
        assert_eq!(d2.year, 2025);
        assert_eq!(d2.month, 6);
        assert_eq!(d2.day, 15);
    }

    #[test]
    fn test_subtract_days() {
        let d = SimpleDate {
            year: 2025,
            month: 1,
            day: 10,
        };
        let d2 = d.subtract_days(10);
        assert_eq!(d2.format(), "2024-12-31");
    }

    #[test]
    fn test_error_display() {
        let err = ExchangeRateAPIError::ApiError {
            status: 401,
            message: "Unauthorized".to_string(),
        };
        assert_eq!(format!("{}", err), "API error (401): Unauthorized");

        let err = ExchangeRateAPIError::ParseError("bad json".to_string());
        assert_eq!(format!("{}", err), "Parse error: bad json");
    }

    #[test]
    fn test_client_creation() {
        let client = ExchangeRateAPI::new("era_live_test123");
        assert_eq!(client.api_key, "era_live_test123");
    }
}
