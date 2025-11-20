use std::ops::Range;

use bts::engine::{Candle, CandleBuilder};

/// Generates deterministic candle data.
pub fn generate_sample_candles(range: Range<i32>, seed: i32, base_price: f64) -> Vec<Candle> {
    range
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

            CandleBuilder::builder()
                .open(open)
                .high(high)
                .low(low)
                .close(close)
                .volume(volume)
                .bid(bid)
                .build()
                .unwrap()
        })
        .collect()
}

#[allow(dead_code)]
fn main() {}
