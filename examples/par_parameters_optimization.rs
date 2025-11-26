//! # Parallel EMA and MACD Parameters Optimization
//!
//! This module implements a **parallel brute-force optimization** to find optimal
//! EMA and MACD parameters for trading strategies using multi-threading.

mod utils;

use bts::prelude::*;
use rayon::prelude::*;
use ta::{indicators::*, *};

use utils::*;

const START: usize = 8;
const END: usize = 13;

#[derive(Debug, PartialEq)]
struct Parameters(usize, (usize, usize, usize), f64);

impl From<(usize, (usize, usize, usize), f64)> for Parameters {
    fn from(value: (usize, (usize, usize, usize), f64)) -> Self {
        Self(value.0, value.1, value.2)
    }
}

impl PartialOrd for Parameters {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.2.partial_cmp(&other.2)
    }
}

impl std::fmt::Display for Parameters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ema = self.0;
        let m1 = self.1.0;
        let m2 = self.1.1;
        let m3 = self.1.2;
        let b = self.2;
        write!(f, "EMA: {ema:3}, MACD: ({m1:3}, {m2:3}, {m3:3}) | Balance: ${b:.2}")
    }
}

impl Parameters {
    fn balance(&self) -> f64 {
        self.2
    }
}

fn main() -> anyhow::Result<()> {
    if START > END {
        return Err(anyhow::Error::msg("END must be greater than START"));
    }

    let candles = utils::generate_sample_candles(3000, 42, 100.0);
    let initial_balance = 1_000.0;
    let min = START;
    let max = END;
    let total_iterations = (max - min + 1_usize).pow(4);

    let shared = SharedResults::new(total_iterations);

    // Collect all parameter combinations
    let params: Vec<(usize, usize, usize, usize)> = (min..=max)
        .flat_map(|macd1| {
            (min..=max).flat_map(move |macd2| {
                (min..=max).flat_map(move |macd3| (min..=max).map(move |ema| (ema, macd1, macd2, macd3)))
            })
        })
        .collect();

    // Process in parallel
    params.par_chunks(1000).for_each(|chunk| {
        let mut bt = Backtest::new(candles.clone(), initial_balance, None).unwrap();

        for &(ema_period, macd1, macd2, macd3) in chunk {
            shared.increment_iter();
            shared.print_progress();

            let mut ema = ExponentialMovingAverage::new(ema_period).unwrap();
            let mut macd = MovingAverageConvergenceDivergence::new(macd1, macd2, macd3).unwrap();

            let result = bt.run(|bt, candle| {
                let close = candle.close();
                let output = ema.next(close);
                let MovingAverageConvergenceDivergenceOutput { histogram, .. } = macd.next(close);

                let balance = bt.free_balance()?;
                let amount = balance.how_many(2.0).max(21.0);

                if balance > (initial_balance / 2.0) && close > output && histogram > 0.0 {
                    let quantity = amount / close;
                    let order = (
                        OrderType::Market(close),
                        OrderType::TrailingStop(close, 2.0),
                        quantity,
                        OrderSide::Buy,
                    );
                    bt.place_order(order.into())?;
                }
                Ok(())
            });

            let current_balance = bt.total_balance();
            let parameters = Parameters::from((ema_period, (macd1, macd2, macd3), current_balance));

            match result {
                Ok(_) => shared.push_result(parameters, false),
                Err(_) => shared.push_result(parameters, true),
            }

            bt.reset();
        }
    });

    println!("\n\nPARAMETERS: MIN {START}, MAX {END}, NB TICKS {}", candles.len());
    println!("\n=== TOP {} EMA/MACD Parameters ===", utils::CAPACITY);
    for p in shared.total_balances().lock().unwrap().iter() {
        let opt = (p.balance() - initial_balance) / initial_balance * 100.0;
        println!("{p} ({opt:+.2}%)");
    }

    println!("\n=== ERROR CASES (TOP {}) ===", utils::CAPACITY);
    for p in shared.errors().lock().unwrap().iter() {
        let opt = (p.balance() - initial_balance) / initial_balance * 100.0;
        println!("{p} ({opt:+.2}%)");
    }

    Ok(())
}
