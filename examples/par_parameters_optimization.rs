//! # Parallel EMA and MACD Parameters Optimization
//!
//! This module implements a **parallel brute-force optimization** to find optimal
//! EMA and MACD parameters for trading strategies using multi-threading.

mod utils;

use bts_rs::prelude::*;
use ta::{indicators::*, *};

const START: usize = 8;
const END: usize = 13;

#[derive(Clone)]
struct Parameters;

impl ParameterCombination for Parameters {
    type Output = (usize, usize, usize, usize);

    fn generate() -> Vec<Self::Output> {
        let min = START;
        let max = END;
        (min..=max)
            .flat_map(|macd1| {
                (min..=max).flat_map(move |macd2| {
                    (min..=max).flat_map(move |macd3| (min..=max).map(move |ema| (ema, macd1, macd2, macd3)))
                })
            })
            .collect()
    }
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let candles = utils::example_candles();
    let initial_balance = 1_000.0;
    let opt = Optimizer::<Parameters>::new(candles.clone(), initial_balance, None);

    let result = opt.with(
        |&(ema_period, m1, m2, m3)| {
            let ema = ExponentialMovingAverage::new(ema_period).map_err(|e| Error::Msg(e.to_string()))?;
            let macd = MovingAverageConvergenceDivergence::new(m1, m2, m3).map_err(|e| Error::Msg(e.to_string()))?;
            Ok((ema, macd))
        },
        |bt, (ema, macd), candle| {
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
        },
    )?;

    let mut result_with_sum = result
        .into_iter()
        .map(|((ema, m1, m2, m3), balance)| {
            let sum = ema + m1 + m2 + m3;
            ((ema, m1, m2, m3), balance, sum)
        })
        .collect::<Vec<_>>();

    result_with_sum.sort_by(|(_, balance1, sum1), (_, balance2, sum2)| {
        balance2
            .partial_cmp(balance1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(sum1.cmp(sum2))
    });

    let top = result_with_sum
        .into_iter()
        .filter(|(_, balance, _)| *balance > initial_balance)
        .take(5)
        .collect::<Vec<_>>();

    println!("\n\nPARAMETERS: MIN {START}, MAX {END}, NB TICKS {}", candles.len());
    println!("\n=== TOP {} EMA/MACD Parameters ===", top.len());
    for (parameters, balance, sum) in top {
        let opt = initial_balance.change(balance);
        println!("{parameters:?} ({sum}) | {balance:.3} ({opt:+.2}%)");
    }

    Ok(())
}
