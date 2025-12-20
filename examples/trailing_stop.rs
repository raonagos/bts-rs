//! # Turtle Trading Strategy with Trailing Stop
//!
//! This example implements a simplified version of the famous **Turtle Trading Strategy**
//! developed by Richard Dennis, which uses trend-following techniques with strict risk management.
mod utils;

use std::{error::Error, sync::Arc};

use bts_rs::prelude::*;
use ta::{indicators::*, *};

fn main() -> Result<(), Box<dyn Error>> {
    let data = utils::example_candles();
    let initial_balance = 1_000.0;
    let candles = Arc::from_iter(data);
    let mut bts = Backtest::new(candles.clone(), initial_balance, None)?;
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
            // .draw_output(DrawOutput::Html("bts.html".to_owned()))
            .draw_output(DrawOutput::Svg("bts.svg".to_owned()))
            .show_volume(true);
        #[cfg(feature = "metrics")]
        let options = options.show_metrics(true);
        let draw = Draw::from(&bts).with_options(options);
        draw.plot()?;
    }

    Ok(())
}
