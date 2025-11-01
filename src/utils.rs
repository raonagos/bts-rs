use std::{fs::File, io::BufReader, path::PathBuf};

use anyhow::{Error, Result};
use chrono::{DateTime, Utc, serde::ts_microseconds};
use serde::Deserialize;

// "open_time": 1759813200000,
// "open_price": 124499.99,
// "high_price": 124640.76,
// "low_price": 124240.37,
// "close_price": 124414.17,
// "volume": 424.20697,
// "close_time": 1759816799999,
// "quote_asset_volume": 52795455.4981537,
// "number_of_trades": 102055,
// "taker_buy_base_volume": 211.69336,
// "taker_buy_quote_volume": 26344187.9144172,
// "ignore": 0.0

#[derive(Debug, Deserialize, Clone)]
pub struct Data {
    #[serde(alias = "open_price")]
    open: f64,
    #[serde(alias = "high_price")]
    high: f64,
    #[serde(alias = "low_price")]
    low: f64,
    #[serde(alias = "close_price")]
    close: f64,
    #[serde(rename = "quote_asset_volume")]
    volume: f64,
    #[serde(rename = "taker_buy_quote_volume")]
    bid: f64,
    #[serde(with = "ts_microseconds")]
    open_time: DateTime<Utc>,
    #[serde(with = "ts_microseconds")]
    close_time: DateTime<Utc>,
}

impl Data {
    pub fn open(&self) -> f64 {
        self.open
    }

    pub fn high(&self) -> f64 {
        self.high
    }

    pub fn low(&self) -> f64 {
        self.low
    }

    pub fn close(&self) -> f64 {
        self.close
    }

    pub fn volume(&self) -> f64 {
        self.volume
    }

    pub fn ask(&self) -> f64 {
        self.volume - self.bid
    }

    pub fn bid(&self) -> f64 {
        self.bid
    }

    pub fn open_time(&self) -> DateTime<Utc> {
        self.open_time
    }

    pub fn close_time(&self) -> DateTime<Utc> {
        self.close_time
    }
}

pub fn get_data_from_file(filepath: PathBuf) -> Result<Vec<Data>> {
    let file = File::open(filepath)?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|e| Error::msg(e.to_string()))
}
