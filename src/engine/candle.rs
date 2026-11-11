use crate::errors::{Error, Result};

use chrono::{DateTime, Utc};

/// Represents a financial candle (or candlestick): OHLC prices, total volume, and
/// the volume traded on the bid side.
///
/// `volume` is the total volume over the candle's period; `bid` is the part of it
/// that traded on the bid side. The ask-side volume is the remainder,
/// `volume - bid`, exposed by [`Candle::ask`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone, Copy)]
pub struct Candle {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    bid: f64,
    open_time: DateTime<Utc>,
    close_time: DateTime<Utc>,
}

impl Candle {
    /// Returns the opening price of the candle.
    pub fn open(&self) -> f64 {
        self.open
    }

    /// Returns the highest price reached during the candle period.
    pub fn high(&self) -> f64 {
        self.high
    }

    /// Returns the lowest price reached during the candle period.
    pub fn low(&self) -> f64 {
        self.low
    }

    /// Returns the closing price of the candle.
    pub fn close(&self) -> f64 {
        self.close
    }

    /// Returns the trading volume during the candle period.
    pub fn volume(&self) -> f64 {
        self.volume
    }

    /// Returns the volume traded on the bid side during the candle.
    pub fn bid(&self) -> f64 {
        self.bid
    }

    /// Returns the volume traded on the bid side during the candle.
    ///
    /// Alias of [`bid`](Self::bid), with the unit in the name.
    pub fn bid_volume(&self) -> f64 {
        self.bid
    }

    /// Returns the volume traded on the ask side during the candle.
    ///
    /// This is `volume() - bid()`. It is a volume, not a price.
    pub fn ask(&self) -> f64 {
        self.volume - self.bid
    }

    /// Returns the volume traded on the ask side during the candle.
    ///
    /// Alias of [`ask`](Self::ask), with the unit in the name.
    pub fn ask_volume(&self) -> f64 {
        self.volume - self.bid
    }

    /// Returns the open time of the candle.
    pub fn open_time(&self) -> DateTime<Utc> {
        self.open_time
    }

    /// Returns the close time of the candle.
    pub fn close_time(&self) -> DateTime<Utc> {
        self.close_time
    }

    /// Checks if the candle is bullish (close > open).
    pub fn is_bullish(&self) -> bool {
        self.close > self.open
    }

    /// Checks if the candle is bearish (close < open).
    pub fn is_bearish(&self) -> bool {
        self.close < self.open
    }
}

/// Builder for creating validated `Candle` instances.
#[derive(Debug)]
pub struct CandleBuilder {
    open: Option<f64>,
    high: Option<f64>,
    low: Option<f64>,
    close: Option<f64>,
    volume: Option<f64>,
    bid: Option<f64>,
    open_time: Option<DateTime<Utc>>,
    close_time: Option<DateTime<Utc>>,
}

impl Default for CandleBuilder {
    fn default() -> Self {
        Self::builder()
    }
}

impl CandleBuilder {
    /// Creates a new `CandleBuilder`.
    pub fn builder() -> Self {
        Self {
            open: None,
            high: None,
            low: None,
            close: None,
            volume: None,
            bid: None,
            open_time: None,
            close_time: None,
        }
    }

    /// Sets the open price.
    pub fn open(mut self, open: f64) -> Self {
        self.open = Some(open);
        self
    }

    /// Sets the high price.
    pub fn high(mut self, high: f64) -> Self {
        self.high = Some(high);
        self
    }

    /// Sets the low price.
    pub fn low(mut self, low: f64) -> Self {
        self.low = Some(low);
        self
    }

    /// Sets the close price.
    pub fn close(mut self, close: f64) -> Self {
        self.close = Some(close);
        self
    }

    /// Sets the volume.
    pub fn volume(mut self, volume: f64) -> Self {
        self.volume = Some(volume);
        self
    }

    /// Sets the bid-side volume.
    pub fn bid(mut self, bid: f64) -> Self {
        self.bid = Some(bid);
        self
    }

    /// Sets the open time.
    pub fn open_time(mut self, ot: DateTime<Utc>) -> Self {
        self.open_time = Some(ot);
        self
    }
    /// Sets the close time.
    pub fn close_time(mut self, ct: DateTime<Utc>) -> Self {
        self.close_time = Some(ct);
        self
    }

    /// Builds a `Candle` after validating the data.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Any required field is missing (`open`, `high`, `low`, `close`, `volume`,
    ///   `open_time`, `close_time`)
    /// - A price is negative, or `low` is not the minimum and `high` the maximum
    ///   of the four prices
    /// - Volume is negative
    /// - The bid-side volume is negative or greater than the total volume
    /// - `open_time` is after `close_time`
    pub fn build(self) -> Result<Candle> {
        // Check required fields
        let open = self.open.ok_or(Error::MissingField("open"))?;
        let high = self.high.ok_or(Error::MissingField("high"))?;
        let low = self.low.ok_or(Error::MissingField("low"))?;
        let close = self.close.ok_or(Error::MissingField("close"))?;
        let volume = self.volume.ok_or(Error::MissingField("volume"))?;
        let open_time = self.open_time.ok_or(Error::MissingField("open time"))?;
        let close_time = self.close_time.ok_or(Error::MissingField("close time"))?;

        // Validate prices
        if !(low <= open && low <= close && low <= high && high >= open && high >= close && low >= 0.0) {
            return Err(Error::InvalidPriceOrder(open, low, high, close));
        }

        // Validate volume
        if volume < 0.0 {
            return Err(Error::NegativeVolume(volume));
        }

        let bid = self.bid.unwrap_or(0.0);
        if !(0.0..=volume).contains(&bid) {
            return Err(Error::InvalidBidVolume(bid, volume));
        }

        // Validate times
        if open_time > close_time {
            return Err(Error::InvalidTimes(open_time, close_time));
        }

        Ok(Candle {
            open,
            high,
            low,
            close,
            volume,
            bid, // 0.0 if not provided
            open_time,
            close_time,
        })
    }
}

#[cfg(test)]
#[test]
fn candle_accessors() {
    let candle = CandleBuilder::builder()
        .open(100.0)
        .high(110.0)
        .low(95.0)
        .close(105.0)
        .volume(1000.0)
        .bid(104.5)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build()
        .unwrap();

    assert_eq!(candle.open(), 100.0);
    assert_eq!(candle.high(), 110.0);
    assert_eq!(candle.low(), 95.0);
    assert_eq!(candle.close(), 105.0);
    assert_eq!(candle.volume(), 1000.0);
    assert_eq!(candle.bid(), 104.5);
    assert_eq!(candle.ask(), 1000.0 - 104.5); // volume - bid
    assert_eq!(candle.bid_volume(), 104.5);
    assert_eq!(candle.ask_volume(), 1000.0 - 104.5);
    assert!(candle.open_time() < candle.close_time())
}

#[cfg(test)]
#[test]
fn candle_type_detection() {
    let bullish = CandleBuilder::builder()
        .open(100.0)
        .high(110.0)
        .low(95.0)
        .close(105.0)
        .volume(1000.0)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build()
        .unwrap();
    let bearish = CandleBuilder::builder()
        .open(105.0)
        .high(110.0)
        .low(95.0)
        .close(100.0)
        .volume(1000.0)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build()
        .unwrap();
    let neutral = CandleBuilder::builder()
        .open(100.0)
        .high(100.0)
        .low(100.0)
        .close(100.0)
        .volume(1000.0)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build()
        .unwrap();

    assert!(bullish.is_bullish());
    assert!(!bullish.is_bearish());

    assert!(!bearish.is_bullish());
    assert!(bearish.is_bearish());

    assert!(!neutral.is_bullish());
    assert!(!neutral.is_bearish());
}

#[cfg(test)]
#[test]
fn candle_builder_valid() {
    let candle = CandleBuilder::builder()
        .open(100.0)
        .high(110.0)
        .low(95.0)
        .close(105.0)
        .volume(1000.0)
        .bid(104.5)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build()
        .unwrap();

    assert_eq!(candle.open(), 100.0);
    assert_eq!(candle.high(), 110.0);
    assert_eq!(candle.low(), 95.0);
    assert_eq!(candle.close(), 105.0);
    assert_eq!(candle.volume(), 1000.0);
    assert_eq!(candle.bid(), 104.5);
}

#[cfg(test)]
#[test]
fn candle_builder_missing_fields() {
    // Missing open
    let result = CandleBuilder::builder()
        .high(110.0)
        .low(95.0)
        .close(105.0)
        .volume(1000.0)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build();
    assert!(matches!(result, Err(Error::MissingField("open"))));

    // Missing high
    let result = CandleBuilder::builder()
        .open(100.0)
        .low(95.0)
        .close(105.0)
        .volume(1000.0)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build();
    assert!(matches!(result, Err(Error::MissingField("high"))));

    // Missing low
    let result = CandleBuilder::builder()
        .open(100.0)
        .high(110.0)
        .close(105.0)
        .volume(1000.0)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build();
    assert!(matches!(result, Err(Error::MissingField("low"))));

    // Missing close
    let result = CandleBuilder::builder()
        .open(100.0)
        .high(110.0)
        .low(95.0)
        .volume(1000.0)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build();
    assert!(matches!(result, Err(Error::MissingField("close"))));

    // Missing volume
    let result = CandleBuilder::builder()
        .open(100.0)
        .high(110.0)
        .low(95.0)
        .close(105.0)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build();
    assert!(matches!(result, Err(Error::MissingField("volume"))));

    // Missing open time
    let result = CandleBuilder::builder()
        .open(100.0)
        .high(110.0)
        .low(95.0)
        .close(105.0)
        .volume(1000.0)
        .build();
    assert!(matches!(result, Err(Error::MissingField("open time"))));

    // Missing close time
    let result = CandleBuilder::builder()
        .open(100.0)
        .high(110.0)
        .low(95.0)
        .close(105.0)
        .volume(1000.0)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .build();
    assert!(matches!(result, Err(Error::MissingField("close time"))));
}

#[cfg(test)]
#[test]
fn candle_builder_invalid_prices() {
    // open > high
    let result = CandleBuilder::builder()
        .open(110.0)
        .high(100.0)
        .low(95.0)
        .close(105.0)
        .volume(1000.0)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build();
    assert!(matches!(result, Err(Error::InvalidPriceOrder { .. })));

    // low > high
    let result = CandleBuilder::builder()
        .open(100.0)
        .high(105.0)
        .low(110.0)
        .close(105.0)
        .volume(1000.0)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build();
    assert!(matches!(result, Err(Error::InvalidPriceOrder { .. })));

    // high < close
    let result = CandleBuilder::builder()
        .open(100.0)
        .high(105.0)
        .low(95.0)
        .close(110.0)
        .volume(1000.0)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build(); // OK: 105 < 110
    assert!(matches!(result, Err(Error::InvalidPriceOrder { .. })));

    // open < low
    let result = CandleBuilder::builder()
        .open(100.0)
        .high(110.0)
        .low(105.0)
        .close(105.0)
        .volume(1000.0)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build();
    assert!(matches!(result, Err(Error::InvalidPriceOrder { .. })));
}

#[cfg(test)]
#[test]
fn candle_builder_negative_volume() {
    let result = CandleBuilder::builder()
        .open(100.0)
        .high(110.0)
        .low(95.0)
        .close(105.0)
        .volume(-1000.0)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build();
    assert!(matches!(result, Err(Error::NegativeVolume(-1000.0))));
}

#[cfg(test)]
#[test]
fn candle_builder_optional_bid() {
    let candle = CandleBuilder::builder()
        .open(100.0)
        .high(110.0)
        .low(95.0)
        .close(105.0)
        .volume(1000.0)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build()
        .unwrap();
    assert_eq!(candle.bid(), 0.0);

    let candle = CandleBuilder::builder()
        .open(100.0)
        .high(110.0)
        .low(95.0)
        .close(105.0)
        .volume(1000.0)
        .bid(104.5)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build()
        .unwrap();
    assert_eq!(candle.bid(), 104.5);
}

#[cfg(test)]
#[test]
fn candle_builder_chaining() {
    let candle = CandleBuilder::builder()
        .open(100.0)
        .high(110.0)
        .low(95.0)
        .close(105.0)
        .volume(1000.0)
        .bid(104.5)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build()
        .unwrap();

    assert_eq!(candle.open(), 100.0);
    assert_eq!(candle.bid(), 104.5);
}

#[cfg(test)]
#[test]
fn candle_ask_calculation() {
    let candle = CandleBuilder::builder()
        .open(100.0)
        .high(110.0)
        .low(95.0)
        .close(105.0)
        .volume(1000.0)
        .bid(104.5)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build()
        .unwrap();
    assert_eq!(candle.ask(), 1000.0 - 104.5);

    let candle = CandleBuilder::builder()
        .open(100.0)
        .high(110.0)
        .low(95.0)
        .close(105.0)
        .volume(1000.0)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build()
        .unwrap();
    assert_eq!(candle.ask(), 1000.0 - 0.0);
}

#[cfg(test)]
#[test]
fn candle_builder_invalid_bid_volume() {
    let open_time = DateTime::from_timestamp_secs(1515151515).unwrap();
    let close_time = DateTime::from_timestamp_secs(1515151516).unwrap();

    let result = CandleBuilder::builder()
        .open(100.0)
        .high(110.0)
        .low(95.0)
        .close(105.0)
        .volume(1000.0)
        .bid(-1.0)
        .open_time(open_time)
        .close_time(close_time)
        .build();
    assert!(matches!(result, Err(Error::InvalidBidVolume(-1.0, 1000.0))));

    let result = CandleBuilder::builder()
        .open(100.0)
        .high(110.0)
        .low(95.0)
        .close(105.0)
        .volume(1000.0)
        .bid(1000.1)
        .open_time(open_time)
        .close_time(close_time)
        .build();
    assert!(matches!(result, Err(Error::InvalidBidVolume(_, _))));
}

#[cfg(test)]
#[test]
fn candle_builder_invalid_time_order() {
    let result = CandleBuilder::builder()
        .open(100.0)
        .high(110.0)
        .low(95.0)
        .close(105.0)
        .volume(1000.0)
        .open_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .build();

    assert!(matches!(result, Err(Error::InvalidTimes(_, _))));
}
