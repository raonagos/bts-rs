use bts::prelude::*;

use ta::indicators::MovingAverageConvergenceDivergence;
use ta::indicators::MovingAverageConvergenceDivergenceOutput;
use ta::{indicators::ExponentialMovingAverage, *};

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

    while let Some(candle) = bt.next() {
        let close = candle.close();
        let output = ema.next(close);
        let MovingAverageConvergenceDivergenceOutput { histogram, .. } = macd.next(close);

        if close > output && histogram > 0.0 {
            let quantity = bt.current_balance().how_many(15.0) / close;
            let position = (
                PositionSide::Long,
                close,
                quantity,
                PositionExitRule::Limit(close),
            );
            _ = bt.open_position(position.into());
        }

        bt.open_positions()
            .iter()
            .filter(|p| {
                let profit = p.profit_change(close);
                // more than 5%
                profit > 5.0
            })
            .for_each(|p| {
                _ = bt.close_position(p.id(), close);
            });
    }

    let fc = candles.first().unwrap();
    let lc = candles.last().unwrap();
    let n = candles.len();

    let exit_price = lc.close();
    if let Result::Ok(sum) = bt.close_all_positions(exit_price) {
        println!("close all positions sum: {sum:.2}");
    }

    let new_balance = bt.current_balance();
    let performance = (new_balance - initial_balance) / initial_balance * 100.0;
    let fc_quant = initial_balance / fc.close();
    let lc_cost = lc.close() * fc_quant;
    let buy_and_hold_performance = (lc_cost - initial_balance) / initial_balance * 100.0;
    let count_position = bt.position_history().len();

    println!("initial balance {initial_balance}");
    println!("new balance {new_balance:.3} USD\ntrades {count_position} / total ticks {n}");
    println!(
        "performance {performance:.3}%\nbuy and hold {buy_and_hold_performance:.3}% ({lc_cost:.3} USD)"
    );

    Ok(())
}
