#![allow(dead_code)]

use std::cmp::Ordering;
use std::io::Write;
use std::sync::Mutex;

use bts::engine::{Candle, CandleBuilder};
#[cfg(feature = "metrics")]
use bts::metrics::Metrics;
use chrono::{DateTime, Duration};

pub const CAPACITY: usize = 5;

pub struct SharedResults<T> {
    total_balances: Mutex<Vec<T>>,
    errors: Mutex<Vec<T>>,
    current_iter: Mutex<usize>,
    total_iterations: usize,
}

impl<T: PartialOrd> SharedResults<T> {
    pub fn new(total: usize) -> Self {
        Self {
            total_balances: Mutex::new(Vec::<T>::with_capacity(CAPACITY)),
            errors: Mutex::new(Vec::<T>::with_capacity(CAPACITY)),
            current_iter: Mutex::new(0),
            total_iterations: total,
        }
    }

    pub fn total_balances(&self) -> &Mutex<Vec<T>> {
        &self.total_balances
    }

    pub fn errors(&self) -> &Mutex<Vec<T>> {
        &self.errors
    }

    pub fn push_result(&self, item: T, is_error: bool) {
        let mut guard = if is_error {
            self.errors.lock().unwrap()
        } else {
            self.total_balances.lock().unwrap()
        };

        guard.push(item);
        guard.sort_by(|a, b| b.partial_cmp(a).unwrap_or(Ordering::Equal));
        if guard.len() > CAPACITY {
            guard.truncate(CAPACITY);
        }
    }

    pub fn increment_iter(&self) -> usize {
        let mut iter = self.current_iter.lock().unwrap();
        *iter += 1;
        *iter
    }

    pub fn print_progress(&self) {
        let iter = *self.current_iter.lock().unwrap();
        let progress = (iter as f64 / self.total_iterations as f64) * 100.0;
        print!("\rProgress: {progress:.2}% ({iter}/{})", self.total_iterations);
        std::io::stdout().flush().unwrap();
    }
}

/// Generates deterministic candle data.
pub fn generate_sample_candles(max: i32, seed: i32, base_price: f64) -> Vec<Candle> {
    let mut open_time = DateTime::default();

    (0..=max)
        .map(|i| {
            // Base price with trend (+ 0.5*i)
            let base_price = base_price + 0.5 * (i as f64);

            // Price variation using simple trigonometric function with seed
            let variation = 5.0 * ((i as f64 * 0.3 + seed as f64).sin() * 0.5 + 0.5);

            // Calculate OHLC prices
            let close = base_price + variation;
            let open = if i == 0 { close - 1.0 } else { close - 0.5 * variation };
            let high = close + 0.3 * variation.abs();
            let low = close - 0.3 * variation.abs();
            // Ensure valid price order: open ≤ low ≤ high ≤ close
            let low = low.min(open);
            let high = high.max(close);
            // Volume with seasonal pattern
            let volume = 1000.0 + 500.0 * ((i as f64 * 0.2).sin()).abs();
            // Bid price (slightly below close)
            let bid = close * 0.999;

            let close_time = open_time + Duration::days(1);

            let candle = CandleBuilder::builder()
                .open(open)
                .high(high)
                .low(low)
                .close(close)
                .volume(volume)
                .bid(bid)
                .open_time(open_time)
                .close_time(close_time)
                .build()
                .unwrap();

            open_time = close_time + Duration::microseconds(1);
            candle
        })
        .collect()
}

/// Pretty print Metrics
#[cfg(feature = "metrics")]
pub fn print_metrics(metrics: &Metrics, initial_balance: f64) {
    println!("=== Backtest Metrics ===");
    println!("Initial Balance: {:.2}", initial_balance);
    println!("Max Drawdown: {:.2}%", metrics.max_drawdown());
    println!("Profit Factor: {:.2}", metrics.profit_factor());
    println!("Sharpe Ratio (risk-free rate = 2%): {:.2}", metrics.sharpe_ratio(0.02));
    println!("Win Rate: {:.2}%", metrics.win_rate());
}

fn main() {}
