//! # Turtle Trading Strategy with Take profit and Stop loss
//!
//! This example implements a simplified version of the famous **Turtle Trading Strategy**
//! developed by Richard Dennis, which uses trend-following techniques with strict risk management.

mod utils;

use bts::prelude::*;
use ta::{
    indicators::{
        ExponentialMovingAverage, MovingAverageConvergenceDivergence, MovingAverageConvergenceDivergenceOutput,
    },
    *,
};

fn main() -> anyhow::Result<()> {
    let candles = utils::generate_sample_candles(0..3000, 42, 100.0);
    let initial_balance = 1_000.0;
    let mut bt = Backtest::new(candles.clone(), initial_balance, None)?;
    let mut ema = ExponentialMovingAverage::new(100)?;
    let mut macd = MovingAverageConvergenceDivergence::default();

    bt.run(|bt, candle| {
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
                // 1/3 RR
                OrderType::TakeProfitAndStopLoss(close.addpercent(6.0), close.subpercent(2.0)),
                quantity,
                OrderSide::Buy,
            );
            bt.place_order(order.into())?;
        }

        Ok(())
    })?;

    #[cfg(feature = "metrics")]
    {
        let metrics = Metrics::from(&bt);
        println!("{metrics}");
    }

    #[cfg(not(feature = "metrics"))]
    {
        let first_price = candles.first().unwrap().close();
        let last_price = candles.last().unwrap().close();

        let n = candles.len();
        println!("trades {n}");

        let new_balance = bt.balance();
        let t_balance = bt.total_balance();
        let new_balance_perf = initial_balance.change(new_balance);
        let t_balance_perf = initial_balance.change(t_balance);
        println!("performance {new_balance:.2}/{t_balance:.2} ({new_balance_perf:.2}%/{t_balance_perf:.2}%)");

        let buy_and_hold = (initial_balance / first_price) * last_price;
        let buy_and_hold_perf = first_price.change(last_price);
        println!("buy and hold {buy_and_hold:.2} ({buy_and_hold_perf:.2}%)");
    }

    Ok(())
}
