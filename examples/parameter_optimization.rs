//! # EMA Parameter Optimization
//!
//! This module implements a **brute-force optimization** to find the optimal EMA (Exponential Moving Average)
//! period for a trading strategy. It tests a range of EMA periods (from 3 to 200) and evaluates which
//! period yields the highest final balance when used in a trend-following strategy.
//!
//! ## Strategy Logic
//! - Uses a **trend-following approach**: Buy when price closes above the EMA
//! - Implements **risk management**: Maximum 2% of capital per trade, minimum trade size of 21 units
//! - Uses **stop loss**: 2% to protect profits and limit losses
//! - Only trades when account balance is above 50% of initial capital (risk control)
//!
//! ## Optimization Process
//! 1. Iterates through EMA periods from 3 to 200
//! 2. For each period:
//!    - Resets the backtest to initial state
//!    - Runs the strategy with the current EMA period
//!    - Records the final balance
//! 3. Sorts results by final balance (descending order)
//! 4. Outputs:
//!    - Top performing periods (successful backtests)
//!    - Error cases (failed backtests)

mod utils;

use std::cmp::Ordering;

use bts_rs::prelude::*;
use ta::{indicators::*, *};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let candles = utils::example_candles();
    let initial_balance = 1_000.0;
    let mut bts = Backtest::new(candles.clone(), initial_balance, None)?;

    let mut total_balances = vec![];
    let mut errors = vec![];

    for period in 3..200 {
        let mut ema = ExponentialMovingAverage::new(period)?;
        let result = bts.run(|bt, candle| {
            let close = candle.close();
            let output = ema.next(close);

            let balance = bt.free_balance()?;
            // 21: minimum to trade
            let amount = balance.how_many(2.0).max(21.0);

            if balance > (initial_balance / 2.0) && close > output {
                let quantity = amount / close;
                let order = (
                    OrderType::Market(close),
                    // 1/3 RR
                    OrderType::TakeProfitAndStopLoss(close.addpercent(6.0), close.subpercent(2.0)),
                    quantity,
                    OrderSide::Buy,
                );
                bt.place_order(order.into())?;
            }

            Ok(())
        });

        match result {
            Ok(_) => total_balances.push((period, bts.total_balance())),
            Err(_) => errors.push((period, bts.total_balance())),
        }

        bts.reset();
    }

    total_balances.sort_by(|(_, a), (_, b)| if a < b { Ordering::Greater } else { Ordering::Less });
    errors.sort_by(|(_, a), (_, b)| if a < b { Ordering::Greater } else { Ordering::Less });

    println!("=== TOP 5 EMA PERIODS ===");
    for (p, b) in total_balances.iter().take(5) {
        let opt = initial_balance.change(*b);
        println!("period: {p} balance: {b:.2} ({opt:.2}%)");
    }

    println!("\n=== ERROR CASES (TOP 5) ===");
    for (p, b) in errors.iter().take(5) {
        let opt = initial_balance.change(*b);
        println!("period: {p} balance: {b:.2} ({opt:.2}%)");
    }

    Ok(())
}
