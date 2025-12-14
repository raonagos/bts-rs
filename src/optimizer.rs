//! Strategy parameter optimization.
//!
//! This module provides tools to optimize trading strategies by testing different parameter combinations.
//! The `Optimizer` struct handles the execution of backtests for each combination, while the
//! `ParameterCombination` trait defines how to generate parameter sets.
//!
//! It needs to enable `optimizer` feature to use it. Take a look at [parallelize parameters optimization](https://github.com/raonagos/bts-rs/blob/master/examples/par_parameters_optimization.rs) for example.

use std::marker::PhantomData;
use std::sync::Arc;

use crate::engine::{Backtest, Candle};
use crate::errors::Result;

use rayon::prelude::*;

/// Trait defining how to generate parameter combinations for optimization.
///
/// Implement this trait for your parameter types to define how combinations should be generated.
/// The associated type `P` represents a single parameter combination (e.g., a tuple of values).
pub trait ParameterCombination: Sync {
    /// Type representing a single parameter combination (e.g., `(usize, f64)`).
    type Item: Clone + Send + Sync;

    /// Generates all possible parameter combinations to test.
    ///
    /// # Returns
    /// A vector containing all parameter combinations.
    fn generate() -> Vec<Self::Item>;
}

/// Optimizer for testing trading strategies with different parameter combinations.
///
/// This struct handles the execution of backtests for each parameter combination,
/// collecting results for analysis.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Optimizer<PC: ParameterCombination> {
    data: Arc<[Candle]>,
    initial_balance: f64,
    _marker: PhantomData<PC>,
    market_fees: Option<(f64, f64)>,
}

impl<PC: ParameterCombination> From<&Backtest> for Optimizer<PC> {
    fn from(value: &Backtest) -> Self {
        Self {
            _marker: PhantomData,
            data: value.candles().cloned().collect(),
            initial_balance: value.initial_balance(),
            market_fees: value.market_fees().copied(),
        }
    }
}

impl<PC: ParameterCombination> Optimizer<PC> {
    /// Creates a new `Optimizer` with the given data and initial balance.
    ///
    /// # Arguments
    /// * `data` - Historical candle data for backtesting.
    /// * `initial_balance` - Starting balance for the backtest.
    /// * `market_fees` - Optional tuple of (maker fee, taker fee).
    ///
    /// # Returns
    /// A new `Optimizer` instance.
    pub fn new(data: Arc<[Candle]>, initial_balance: f64, market_fees: Option<(f64, f64)>) -> Self {
        Self {
            data,
            market_fees,
            initial_balance,
            _marker: PhantomData,
        }
    }

    /// Optimizes a trading strategy by testing all parameter combinations and filtering the results.
    ///
    /// This function leverages multi-threading to evaluate multiple parameter combinations
    /// simultaneously, significantly improving performance by utilizing all available CPU cores.
    ///
    /// # Arguments
    /// * `combinator` - A function that converts a parameter combination into strategy-specific parameters.
    /// * `strategy` - A trading strategy function to test.
    /// * `filter` - A function that takes a reference to a `Backtest` instance after strategy execution and returns an `Option<R>`. The function returns only the `Some` result.
    ///
    /// # Returns
    /// A vector of tuples where each tuple contains:
    /// - The original parameter combination.
    /// - The filtered result, as determined by the `filter` function.
    ///
    /// # Errors
    /// Returns an error if backtest execution fails.
    pub fn with_filter<T, R, C, S, F>(&self, combinator: C, strategy: S, filter: F) -> Result<Vec<(PC::Item, R)>>
    where
        T: Clone,
        R: Clone + Send,
        C: Fn(&PC::Item) -> Result<T> + Sync,
        S: FnMut(&mut Backtest, &mut T, &Candle) -> Result<()> + Clone + Send + Sync,
        F: Fn(&Backtest) -> Option<R> + Send + Sync,
    {
        let num_cpus = num_cpus::get();
        let combinations = PC::generate();
        let filter = Arc::new(filter);
        let chunk_size = combinations.len().div_ceil(num_cpus).max(1);

        combinations
            .par_chunks(chunk_size)
            .map::<_, Result<_>>(|par_combinations| {
                let candles = Arc::clone(&self.data);
                let mut backtest = Backtest::new(candles, self.initial_balance, self.market_fees)?;
                let mut local_results = Vec::with_capacity(par_combinations.len());

                let mut strategy = strategy.clone();
                let filter_arc = Arc::clone(&filter);

                for param_set in par_combinations {
                    let mut output = combinator(param_set)?;
                    backtest.run(|bt, candle| strategy(bt, &mut output, candle))?;
                    let result = filter_arc(&backtest);
                    if let Some(r) = result {
                        local_results.push((param_set.clone(), r));
                    }
                    backtest.reset();
                }

                Ok(local_results)
            })
            .collect::<Result<Vec<_>>>()
            .map(|chunks| chunks.into_iter().flatten().collect())
    }

    /// Optimizes a trading strategy by testing all possible parameter combinations.
    ///
    /// # Arguments
    /// * `combinator` - Function that converts a parameter combination into strategy-specific parameters.
    /// * `strategy` - Trading strategy function to test.
    ///
    /// # Returns
    /// A vector of tuples containing each parameter combination and the `Backtest` instance.
    ///
    /// # Errors
    /// Returns an error if backtest execution fails.
    pub fn with<T, C, S>(&self, combinator: C, strategy: S) -> Result<Vec<(PC::Item, Backtest)>>
    where
        T: Clone,
        C: Fn(&PC::Item) -> Result<T> + Sync,
        S: FnMut(&mut Backtest, &mut T, &Candle) -> Result<()> + Clone + Send + Sync,
    {
        self.with_filter(combinator, strategy, |backtest| Some(backtest.clone()))
    }
}

#[cfg(test)]
#[derive(Clone)]
struct Parameters;

#[cfg(test)]
impl ParameterCombination for Parameters {
    type Item = (usize, usize, usize, usize);

    fn generate() -> Vec<Self::Item> {
        let min = 8;
        let max = 13;
        (min..=max)
            .flat_map(|macd1| {
                (min..=max).flat_map(move |macd2| {
                    (min..=max).flat_map(move |macd3| (min..=max).map(move |ema| (ema, macd1, macd2, macd3)))
                })
            })
            .collect()
    }
}

#[cfg(test)]
fn get_data() -> Vec<Candle> {
    use super::engine::CandleBuilder;
    use chrono::DateTime;

    let candle1 = CandleBuilder::builder()
        .open(90.0)
        .high(110.0)
        .low(80.0)
        .close(100.0)
        .volume(1.0)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build()
        .unwrap();
    let candle2 = CandleBuilder::builder()
        .open(100.0)
        .high(119.0)
        .low(90.0)
        .close(110.0)
        .volume(1.0)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build()
        .unwrap();
    let candle3 = CandleBuilder::builder()
        .open(110.0)
        .high(129.0)
        .low(100.0)
        .close(120.0)
        .volume(1.0)
        .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
        .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
        .build()
        .unwrap();

    vec![candle1, candle2, candle3]
}

#[cfg(test)]
#[test]
fn optimizer_with_ema_macd() {
    use crate::errors::Error;
    use crate::prelude::*;

    use ta::indicators::{
        ExponentialMovingAverage, MovingAverageConvergenceDivergence, MovingAverageConvergenceDivergenceOutput,
    };
    use ta::*;

    let data = get_data();
    let initial_balance = 1_000.0;
    let candles = std::sync::Arc::from_iter(data);

    let opt = Optimizer::<Parameters>::new(candles, initial_balance, None);

    opt.with(
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
                bt.place_order(candle, order.into())?;
            }
            Ok(())
        },
    )
    .unwrap();

    opt.with_filter(
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
                bt.place_order(candle, order.into())?;
            }
            Ok(())
        },
        |b| {
            let orders = b.orders().copied().collect::<Vec<_>>();
            Some(orders)
        },
    )
    .unwrap();
}
