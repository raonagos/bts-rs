//! # Turtle Trading Strategy with Trailing Stop and Multi-Timeframe Analysis
//!
//! This example implements a simplified version of the famous **Turtle Trading Strategy**
//! developed by Richard Dennis, which uses trend-following techniques with strict risk management.

mod utils;

use bts_rs::prelude::*;
use ta::{indicators::*, *};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let candles = utils::example_candles();
    let initial_balance = 1_000.0;
    let mut bts = Backtest::new(candles.clone(), initial_balance, None)?;
    let mut ema = ExponentialMovingAverage::new(100)?;
    let mut macd = MovingAverageConvergenceDivergence::default();

    struct TimeframeAggregator;

    impl Aggregation for TimeframeAggregator {
        fn factors(&self) -> &[usize] {
            &[1, 4, 8]
        }
    }

    bts.run_with_aggregator(&TimeframeAggregator, |bt, candles| {
        let candle_one = candles.first().ok_or(Error::CandleNotFound)?;
        let _candle_four = candles.get(1);
        let _candle_eight = candles.get(2);

        if let Some(_c) = _candle_four {}
        if let Some(_c) = _candle_eight {}

        let close = candle_one.close();
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
            bt.place_order(order.into())?;
        }

        Ok(())
    })?;

    #[cfg(not(feature = "metrics"))]
    {
        let first_price = candles.first().unwrap().close();
        let last_price = candles.last().unwrap().close();

        let n = candles.len();
        println!("trades {n}");

        let new_balance = bts.balance();
        let t_balance = bts.total_balance();
        let new_balance_perf = initial_balance.change(new_balance);
        let t_balance_perf = initial_balance.change(t_balance);
        println!("performance {new_balance:.2}/{t_balance:.2} ({new_balance_perf:.2}%/{t_balance_perf:.2}%)");

        let buy_and_hold = (initial_balance / first_price) * last_price;
        let buy_and_hold_perf = first_price.change(last_price);
        println!("buy and hold {buy_and_hold:.2} ({buy_and_hold_perf:.2}%)");
    }

    #[cfg(feature = "metrics")]
    {
        use crate::utils::print_metrics;

        let metrics = Metrics::from(&bts);
        print_metrics(&metrics, initial_balance);
    }

    #[cfg(feature = "draws")]
    {
        let options = DrawOptions::default()
            .draw_output(DrawOutput::Svg("../bts.svg"))
            .show_volume(true);
        let draw = Draw::with_backtest(&bts).with_options(options);
        draw.plot()?;
    }

    Ok(())
}
