//! Error types for the BTS library.

use chrono::{DateTime, Utc};

/// Enum representing possible errors in the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Custom error types for the `bts` library.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// The candle data provided is empty.
    ///
    /// Backtesting requires at least one candle to execute.
    #[error("Candle data is empty: backtesting requires at least one candle")]
    CandleDataEmpty,

    /// The requested candle was not found in the dataset.
    #[error("Candle not found")]
    CandleNotFound,

    /// The Aggregator factor is invalid.
    #[error("The Aggregator factor is invalid")]
    InvalidFactor,

    /// A required field is missing.
    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    /// Prices are not in valid order (open ≤ low ≤ high ≤ close).
    #[error("Invalid price order: open={0}, low={1}, high={2}, close={3}")]
    InvalidPriceOrder(f64, f64, f64, f64),

    /// Volume cannot be negative.
    #[error("Volume cannot be negative (got: {0})")]
    NegativeVolume(f64),

    /// The bid-side volume is outside `0..=volume`.
    ///
    /// ### Arguments
    /// * `0` - The invalid bid-side volume.
    /// * `1` - The total volume.
    #[error("Bid volume must be between 0 and the total volume (got: {0}, volume: {1})")]
    InvalidBidVolume(f64, f64),

    /// The open time is after the close time.
    #[error("Invalid time order: open={0}, close={1}")]
    InvalidTimes(DateTime<Utc>, DateTime<Utc>),

    /// The open time is after the close time.
    ///
    /// Deprecated spelling of [`InvalidTimes`](Self::InvalidTimes); kept for
    /// source compatibility with earlier releases.
    #[deprecated(since = "1.1.0", note = "use Error::InvalidTimes")]
    #[error("Invalid time order: open={0}, close={1}")]
    InvalideTimes(DateTime<Utc>, DateTime<Utc>),

    /// The initial or current balance is not positive.
    ///
    /// ### Arguments
    /// * `0` - The invalid balance value.
    #[error("Balance must be positive (got: {0})")]
    NegZeroBalance(f64),

    /// The wallet does not have enough funds to place the order.
    ///
    /// ### Arguments
    /// * `0` - The required amount.
    /// * `1` - The available amount.
    #[error("Insufficient funds: required {0}, available {1}")]
    InsufficientFunds(f64, f64),

    /// The free balance is negative.
    ///
    /// ### Arguments
    /// * `0` - The current balance.
    /// * `1` - The locked funds.
    #[error("Negative free balance: balance={0}, locked={1}")]
    NegFreeBalance(f64, f64),

    /// The fees are negative.
    #[error("Negative fees")]
    NegZeroFees,

    /// The locked funds are insufficient for the requested amount.
    ///
    /// ### Arguments
    /// * `0` - The currently locked funds.
    /// * `1` - The requested amount to unlock.
    #[error("Locked funds {0} are insufficient for amount {1}")]
    UnlockBalance(f64, f64),

    /// The requested order was not found.
    #[error("Order not found")]
    OrderNotFound,

    /// Failed to remove an order.
    #[error("Failed to remove order")]
    RemoveOrder,

    /// The requested position was not found.
    #[error("Position not found")]
    PositionNotFound,

    /// Failed to remove a position.
    #[error("Failed to remove position")]
    RemovePosition,

    /// The exit price is invalid.
    #[error("Invalid exit price {0}")]
    ExitPrice(f64),

    /// A generic error with a custom message.
    ///
    /// ### Arguments
    /// * `0` - The error message.
    #[error("{0}")]
    Msg(String),

    /// Take profit or stop loss values must be positive.
    #[error("TakeProfit or StopLoss must be positive")]
    NegTakeProfitAndStopLoss,

    /// Trailing stop values must be positive.
    #[error("TrailingStop must be positive and greater than 0")]
    NegZeroTrailingStop,

    /// The order type is not compatible with the operation.
    ///
    /// Use market or limit orders to open a position, and take profit, stop loss, or trailing stop to close a position.
    #[error("Try another order type")]
    MismatchedOrderType,

    /// An error with plotters crate.
    ///
    /// ### Arguments
    /// * `0` - The underlying plotters error.
    #[cfg(feature = "draws")]
    #[error("{0}")]
    Plotters(String),

    /// An error with charming crate.
    ///
    /// ### Arguments
    /// * `0` - The underlying charming error.
    #[cfg(feature = "draws")]
    #[error("{0}")]
    Charming(#[from] charming::EchartsError),
}

#[cfg(feature = "serde")]
impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Error {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Error proxy
        #[derive(serde::Deserialize)]
        #[serde(tag = "type", content = "data")]
        enum ErrorWrapper {
            CandleDataEmpty,
            CandleNotFound,
            InvalidFactor,
            MissingField {
                field: String,
            },
            InvalidPriceOrder {
                open: f64,
                low: f64,
                high: f64,
                close: f64,
            },
            NegativeVolume {
                volume: f64,
            },
            InvalidBidVolume {
                bid: f64,
                volume: f64,
            },
            InvalidTimes {
                open: i64,
                close: i64,
            },
            InvalideTimes {
                open: i64,
                close: i64,
            },
            NegZeroBalance {
                balance: f64,
            },
            InsufficientFunds {
                required: f64,
                available: f64,
            },
            NegFreeBalance {
                balance: f64,
                locked: f64,
            },
            NegZeroFees,
            UnlockBalance {
                locked: f64,
                amount: f64,
            },
            OrderNotFound,
            RemoveOrder,
            PositionNotFound,
            RemovePosition,
            ExitPrice {
                price: f64,
            },
            Msg {
                message: String,
            },
            NegTakeProfitAndStopLoss,
            NegZeroTrailingStop,
            MismatchedOrderType,
            #[cfg(feature = "draws")]
            Plotters {
                error: String,
            },
            #[cfg(feature = "draws")]
            Charming {
                error: String,
            },
        }

        // Deserialize through the intermediate representation.
        let wrapper = ErrorWrapper::deserialize(deserializer)?;

        // Convert to `Error`.
        Ok(match wrapper {
            ErrorWrapper::CandleDataEmpty => Error::CandleDataEmpty,
            ErrorWrapper::CandleNotFound => Error::CandleNotFound,
            ErrorWrapper::InvalidFactor => Error::InvalidFactor,
            ErrorWrapper::MissingField { field } => match field.as_str() {
                // Keep the `&'static str` variant without leaking memory: map the
                // known builder fields back to their static names and fall back to
                // `Msg` for anything else.
                "open" => Error::MissingField("open"),
                "high" => Error::MissingField("high"),
                "low" => Error::MissingField("low"),
                "close" => Error::MissingField("close"),
                "volume" => Error::MissingField("volume"),
                "open time" => Error::MissingField("open time"),
                "close time" => Error::MissingField("close time"),
                other => Error::Msg(format!("Missing required field: {other}")),
            },
            ErrorWrapper::InvalidPriceOrder { open, low, high, close } => {
                Error::InvalidPriceOrder(open, low, high, close)
            }
            ErrorWrapper::NegativeVolume { volume } => Error::NegativeVolume(volume),
            ErrorWrapper::InvalidBidVolume { bid, volume } => Error::InvalidBidVolume(bid, volume),
            ErrorWrapper::InvalidTimes { open, close } | ErrorWrapper::InvalideTimes { open, close } => {
                let open_dt = DateTime::from_timestamp_millis(open).unwrap_or(Utc::now());
                let close_dt = DateTime::from_timestamp_millis(close).unwrap_or(Utc::now());
                Error::InvalidTimes(open_dt, close_dt)
            }
            ErrorWrapper::NegZeroBalance { balance } => Error::NegZeroBalance(balance),
            ErrorWrapper::InsufficientFunds { required, available } => Error::InsufficientFunds(required, available),
            ErrorWrapper::NegFreeBalance { balance, locked } => Error::NegFreeBalance(balance, locked),
            ErrorWrapper::NegZeroFees => Error::NegZeroFees,
            ErrorWrapper::UnlockBalance { locked, amount } => Error::UnlockBalance(locked, amount),
            ErrorWrapper::OrderNotFound => Error::OrderNotFound,
            ErrorWrapper::RemoveOrder => Error::RemoveOrder,
            ErrorWrapper::PositionNotFound => Error::PositionNotFound,
            ErrorWrapper::RemovePosition => Error::RemovePosition,
            ErrorWrapper::ExitPrice { price } => Error::ExitPrice(price),
            ErrorWrapper::Msg { message } => Error::Msg(message),
            ErrorWrapper::NegTakeProfitAndStopLoss => Error::NegTakeProfitAndStopLoss,
            ErrorWrapper::NegZeroTrailingStop => Error::NegZeroTrailingStop,
            ErrorWrapper::MismatchedOrderType => Error::MismatchedOrderType,
            #[cfg(feature = "draws")]
            ErrorWrapper::Plotters { error } => Error::Plotters(error),
            #[cfg(feature = "draws")]
            ErrorWrapper::Charming { error } => Error::Charming(charming::EchartsError::HtmlRenderingError(error)),
        })
    }
}
