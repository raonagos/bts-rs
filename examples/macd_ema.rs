use bts::prelude::*;

use ta::{
    indicators::{
        ExponentialMovingAverage, MovingAverageConvergenceDivergence,
        MovingAverageConvergenceDivergenceOutput,
    },
    *,
};

fn main() -> anyhow::Result<()> {
    let items = get_data_from_file("data/btc.json".into())?;
    let candles = items
        .iter()
        .map(|d| Candle::from((d.open(), d.high(), d.low(), d.close(), d.volume(), d.bid())))
        .collect::<Vec<_>>();

    let initial_balance = 1_000.0;
    let mut bt = Backtest::new(candles.clone(), initial_balance);
    let mut ema = ExponentialMovingAverage::new(100)?;
    let mut macd = MovingAverageConvergenceDivergence::default();

    let result = bt.run(|bt, candle| {
        let close = candle.close();
        let output = ema.next(close);
        let MovingAverageConvergenceDivergenceOutput { histogram, .. } = macd.next(close);

        if close > output && histogram > 0.0 {
            let quantity = 999.0 / close;
            let order = (
                OrderType::Market(close),
                OrderType::TakeProfitAndStopLoss(close * 2.1, 0.0),
                quantity,
                OrderSide::Buy,
            );
            _ = bt.place_order(order.into());
        }
    });

    if let Err(e) = result {
        return Err(e.into());
    }

    println!("n pos {}", bt.positions.len());
    let last_price = candles.last().unwrap().close();
    _ = bt.close_all_positions(last_price);
    println!("new balance {}", bt.balance());

    Ok(())
}
