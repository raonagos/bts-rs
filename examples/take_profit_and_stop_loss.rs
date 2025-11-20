//! # Turtle Trading Strategy with Take profit and Stop loss
//!
//! This example implements a simplified version of the famous **Turtle Trading Strategy**
//! developed by Richard Dennis, which uses trend-following techniques with strict risk management.
//!
//! ## Key Principles of the Turtle Strategy:
//! - **Risk Management**: Never risk more than 2% of capital on a single trade
//! - **Trend Following**: Use moving averages to identify trends
//!
//! ## Implementation Details:
//! - Uses **100-period EMA** to determine trend direction (price > EMA = uptrend)
//! - Uses **MACD histogram** to confirm momentum (histogram > 0 = bullish)
//! - Implements **2% trailing stop** to manage risk and protect profits
//! - Only trades when account has sufficient free balance (>50% of initial capital)
//! - Calculates position size based on available capital and 2% risk rule
//!
//! ## Strategy Logic:
//! 1. Calculate 100-period EMA and MACD indicators
//! 2. Check if price is above EMA (uptrend) and MACD histogram is positive (momentum)
//! 3. Verify account has sufficient free balance (>50% of initial capital)
//! 4. Calculate position size based on 2% risk rule
//! 5. Enter long position with 2% stop loss
//! 6. Let the take profit or stop loss manage the trade exit
//!
//! ## Risk Management:
//! - Maximum 2% of capital risked per trade (implemented via position sizing)
//! - Take profit and Stop loss protects profits and limits losses
//! - Minimum trade size requirement prevents over-trading

mod data;

use bts::prelude::*;
use ta::{
    indicators::{
        ExponentialMovingAverage, MovingAverageConvergenceDivergence, MovingAverageConvergenceDivergenceOutput,
    },
    *,
};

fn main() -> anyhow::Result<()> {
    let candles = data::generate_sample_candles(0..3000, 42, 100.0);
    let initial_balance = 1_000.0;
    let mut bt = Backtest::new(candles.clone(), initial_balance, None)?;
    let mut ema = ExponentialMovingAverage::new(100)?;
    let mut macd = MovingAverageConvergenceDivergence::default();

    bt.run(|bt, candle| {
        let close = candle.close();
        let output = ema.next(close);
        let MovingAverageConvergenceDivergenceOutput { histogram, .. } = macd.next(close);

        let balance = bt.free_balance()?;
        let amount = balance.how_many(2.0).max(21.0);

        // 21: minimum to trade
        if balance > (initial_balance / 2.0) && close > output && histogram > 0.0 {
            let quantity = amount / close;
            let order = (
                OrderType::Market(close),
                OrderType::TakeProfitAndStopLoss(close * 2.0, close.subpercent(2.0)),
                quantity,
                OrderSide::Buy,
            );
            bt.place_order(order.into())?;
        }

        Ok(())
    })?;

    let first_price = candles.first().unwrap().close();
    let last_price = candles.last().unwrap().close();

    bt.close_all_positions(last_price)?;

    let n = candles.len();
    let close_position_events = bt.events().filter(|e| matches!(e, Event::DelPosition(_))).count();
    println!("trades {close_position_events} / {n}");

    let new_balance = bt.balance();
    let new_balance_perf = initial_balance.change(new_balance);
    println!("performance {new_balance:.2} ({new_balance_perf:.2}%)");

    let buy_and_hold = (initial_balance / first_price) * last_price;
    let buy_and_hold_perf = first_price.change(last_price);
    println!("buy and hold {buy_and_hold:.2} ({buy_and_hold_perf:.2}%)");

    Ok(())
}
