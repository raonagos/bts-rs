use bts::prelude::*;

use ta::{indicators::SimpleMovingAverage, *};

fn main() -> anyhow::Result<()> {
    let items = get_data_from_file("data/btc.json".into())?;
    let candles = items
        .iter()
        .map(|d| Candle::from((d.open(), d.high(), d.low(), d.close(), d.volume(), d.bid())))
        .collect::<Vec<_>>();

    let initial_balance = 1_000.0;
    let mut bt = Backtest::new(candles.clone(), initial_balance);
    let mut sma = SimpleMovingAverage::new(5)?;

    while let Some(candle) = bt.next() {
        let close = candle.close();
        let output = sma.next(close);
        let long_limit = output.subpercent(5.0);
        let low = candle.low();

        if low < long_limit {
            let quantity = bt.current_balance().how_many(15.0) / long_limit;
            let position = (
                PositionSide::Long,
                long_limit,
                quantity,
                PositionExitRule::Market,
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

    let f = candles.first().unwrap();
    let l = candles.last().unwrap();
    let n = candles.len();

    let exit_price = l.close();
    if let Result::Ok(sum) = bt.close_all_positions(exit_price) {
        println!("close all positions sum: {sum}");
    }

    let new_balance = bt.current_balance();
    let performance = (new_balance - initial_balance) / initial_balance * 100.0;
    let f_quant = initial_balance / f.close();
    let l_cost = l.close() * f_quant;
    let buy_and_hold_performance = (l_cost - initial_balance) / initial_balance * 100.0;
    let count_position = bt.position_history().len();

    println!("initial balance {initial_balance}");
    println!("new balance {new_balance:.3} USD\ntrades {count_position} / total ticks {n}");
    println!(
        "performance {performance:.3}%\nbuy and hold {buy_and_hold_performance:.3}% ({l_cost:.3} USD)"
    );

    Ok(())
}
