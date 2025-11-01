use bts::engine::*;
use bts::utils::*;
use bts::*;

use anyhow::*;
use ta::{indicators::SimpleMovingAverage, *};

fn main() -> Result<()> {
    let items = get_data_from_file("data/btc.json".into())?;
    let candles = items
        .iter()
        .map(|d| Candle::from((d.open(), d.high(), d.low(), d.close(), d.volume(), d.bid())))
        .collect::<Vec<_>>();

    let initial_balance = 1_000.0;
    let mut bt = Backtest::new(candles.clone(), initial_balance);
    let mut sma = SimpleMovingAverage::new(5)?;

    use std::result::Result;

    while let Some(candle) = bt.next() {
        let close = candle.close();
        let output = sma.next(close);
        let long_limit = output.subpercent(5.0);
        let (high, low) = (candle.high(), candle.low());

        if low < long_limit {
            let quantity = (15.0 * bt.balance() / 100.0) / long_limit;
            if let Result::Ok(pos) =
                bt.open_position((PositionSide::Long, long_limit, quantity).into())
            {
                println!("opened {}", pos.id());
            }
        }

        if output < high && output > low {
            let open_positions = bt.open_positions();
            open_positions
                .iter()
                .filter(|p| {
                    let entry = p.entry_price() * p.quantity();
                    let profit = p.estimate_profit(output);
                    entry.addpercent(5.0) < profit
                })
                .for_each(|p| {
                    _ = bt.close_position(p.id(), output);
                    println!("closed {}", p.id());
                });
        }
    }

    let f = candles.first().unwrap();
    let l = candles.last().unwrap();
    let buy_and_hold = 100.0 * (initial_balance * l.close() / f.close()) / initial_balance;
    let new_balance = bt.balance();
    let performance = 100.0 * new_balance / initial_balance;
    let performance = if performance < 100.0 {
        -(100.0 - performance)
    } else {
        performance - 100.0
    };
    let count_position = bt.position_history().len();
    println!(
        "new balance {new_balance} USD\ntrades {count_position}\nperformance {performance:.3}%\nbuy and hold {buy_and_hold:.3}%"
    );

    Ok(())
}
