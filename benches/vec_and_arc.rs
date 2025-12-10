//! # Turtle Trading Strategy with Trailing Stop
//!
//! This example implements a simplified version of the famous **Turtle Trading Strategy**
//! developed by Richard Dennis, which uses trend-following techniques with strict risk management.
//!
//! This module benchs `run` function, one with vector and the other with arc of array. Which is the faster ?

mod utils;

use bts_rs::prelude::*;
use criterion::{Criterion, criterion_group, criterion_main};
use ta::{indicators::*, *};

fn run_with_vector(count: usize) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let candles = utils::generate_sample_candles(count, 42, 100.0);
    let initial_balance = 10_000.0;
    let mut bts = Backtest::new(candles, initial_balance, None)?;
    let mut ema = ExponentialMovingAverage::new(100)?;
    let mut macd = MovingAverageConvergenceDivergence::default();

    bts.run(|bt, candle| {
        let close = candle.close();
        let output = ema.next(close);
        let MovingAverageConvergenceDivergenceOutput { histogram, .. } = macd.next(close);

        let balance = bt.free_balance()?;
        // 21: minimum to trade
        let amount = balance.how_many(2.0).max(21.0);

        if balance > (initial_balance / 2.0) && close > output && histogram > 0.0 {
            let quantity = amount / close;
            let order = (
                OrderType::Market(close),
                OrderType::TrailingStop(close, 2.0),
                quantity,
                OrderSide::Buy,
            );
            bt.place_order(candle, order.into())?;
        }

        Ok(())
    })?;

    Ok(())
}

fn run_with_arc(count: usize) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let candles = utils::generate_sample_candles(count, 42, 100.0);
    let initial_balance = 10_000.0;
    let mut bts = Backtest::new(candles, initial_balance, None)?;
    let mut ema = ExponentialMovingAverage::new(100)?;
    let mut macd = MovingAverageConvergenceDivergence::default();

    bts.run_as_arc(|bt, candle| {
        let close = candle.close();
        let output = ema.next(close);
        let MovingAverageConvergenceDivergenceOutput { histogram, .. } = macd.next(close);

        let balance = bt.free_balance()?;
        // 21: minimum to trade
        let amount = balance.how_many(2.0).max(21.0);

        if balance > (initial_balance / 2.0) && close > output && histogram > 0.0 {
            let quantity = amount / close;
            let order = (
                OrderType::Market(close),
                OrderType::TrailingStop(close, 2.0),
                quantity,
                OrderSide::Buy,
            );
            bt.place_order(candle, order.into())?;
        }

        Ok(())
    })?;

    Ok(())
}

fn criterion_benchmark(c: &mut Criterion) {
    use std::hint::black_box;

    for count in [3000, 5000, 7000] {
        c.bench_function(&format!("run with vector ({count})"), |b| {
            b.iter(|| run_with_vector(black_box(count)))
        });
        c.bench_function(&format!("run with arc ({count})"), |b| {
            b.iter(|| run_with_arc(black_box(count)))
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
