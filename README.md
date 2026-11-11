<p align="center">
  <img src="https://raw.githubusercontent.com/raonagos/bts-rs/master/logo.png" alt="BTS logo" width="200"/>
</p>

# BTS — BackTest Strategy

[![Crates.io](https://img.shields.io/crates/v/bts-rs.svg)](https://crates.io/crates/bts-rs)
[![Documentation](https://img.shields.io/docsrs/bts-rs)](https://docs.rs/bts-rs)
[![CI](https://github.com/raonagos/bts-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/raonagos/bts-rs/actions/workflows/rust.yml)
[![License](https://img.shields.io/crates/l/bts-rs)](https://github.com/raonagos/bts-rs/blob/master/LICENSE)
[![Downloads](https://img.shields.io/crates/d/bts-rs)](https://crates.io/crates/bts-rs)
![Rust](https://img.shields.io/badge/rust-1.88%2B-orange)

**BTS** is a Rust library for backtesting trading strategies on OHLCV
candlestick data. It gives you a small, explicit market engine: feed it
candles, write a strategy closure, and the engine maintains orders, positions,
cash, fees, and mark-to-market equity bar by bar.

It is a general-purpose backtesting library. It is not tied to a particular
market, data vendor, or trading system, and it does not ship alpha.

```rust
use std::sync::Arc;

use bts_rs::prelude::*;
use chrono::{DateTime, Duration};

let mut bt = Backtest::new(Arc::from_iter(vec![
    CandleBuilder::builder()
        .open(100.0).high(110.0).low(95.0).close(105.0)
        .volume(1_000.0)
        .open_time(DateTime::default())
        .close_time(DateTime::default() + Duration::days(1))
        .build()
        .unwrap(),
    CandleBuilder::builder()
        .open(105.0).high(115.0).low(100.0).close(112.0)
        .volume(1_000.0)
        .open_time(DateTime::default() + Duration::days(1))
        .close_time(DateTime::default() + Duration::days(2))
        .build()
        .unwrap(),
]), 10_000.0, None).unwrap();

bt.run(|bt, candle| {
    // The order is placed after seeing this candle, so it becomes eligible
    // on the next one.
    let order: Order = (OrderType::Market(candle.close()), 1.0, OrderSide::Buy).into();
    bt.place_order(candle, order)?;
    Ok(())
}).unwrap();

println!("equity: {:.2}", bt.total_balance());
println!("pnl:    {:.2}", bt.total_balance() - bt.initial_balance());
```

## Highlights

- **Bar-by-bar engine** — one event loop, one strategy closure, no hidden
  second pass.
- **Explicit accounting** — cash, position value, equity, fees, and realized
  and unrealized P&L are separate quantities with documented accessors.
- **Order lifecycle** — market and limit entries, take-profit, stop-loss, and
  trailing-stop exits, with pending-order management.
- **No look-ahead by construction** — an order placed after seeing a bar is
  first eligible on a later bar.
- **Metrics** — net P&L, max drawdown, profit factor, win rate, and an
  annualised Sharpe ratio, computed from the equity curve (`metrics` feature).
- **Parameter sweeps** — parallel optimisation over strategy parameters
  (`optimizer` feature).
- **Charts** — SVG, PNG, and HTML candlestick, volume, equity, and metrics
  charts (`draws` feature).
- **Optional serialization** — `serde` support for the engine and event types.
- **Small dependency footprint** — `chrono`, `thiserror`, and `rand`; the
  heavier crates are optional.

## Installation

```toml
[dependencies]
bts-rs = "1.1"

# Recommended for most users: metrics, charts, and parameter sweeps.
# bts-rs = { version = "1.1", features = ["metrics", "draws", "optimizer"] }
```

Or:

```sh
cargo add bts-rs --features metrics
```

## How it works

For every candle, in order:

1. Pending orders whose price falls inside the candle's `low..=high` range are
   filled and become positions.
2. Exit rules on positions opened on earlier bars are evaluated against the
   candle's high and low.
3. Open positions are marked at the candle's close, and the account equity is
   sampled (`Event::WalletUpdate`, with the `metrics` feature).
4. Your strategy receives the candle and may place orders or close positions.

An order placed in step 4 is only considered from step 1 of the next candle.
This is what makes the engine causal: a strategy can read a bar's high and low
without being able to fill against them.

## Orders and positions

| Type | Opens a position | Notes |
| --- | --- | --- |
| `OrderType::Market(price)` | yes | Filled at `price` if the next candle trades through it; otherwise cancelled. |
| `OrderType::Limit(price)` | yes | Stays pending until a candle range contains `price`; filled at `price`. |
| `OrderType::TakeProfitAndStopLoss(tp, sl)` | no | Exit rule. `0.0` disables that side. |
| `OrderType::TrailingStop(price, percent)` | no | Exit rule. The stop trails the best price seen since entry. |

`Order::from((entry_type, quantity, side))` creates an entry order.
`Order::from((entry_type, exit_type, quantity, side))` attaches an exit rule.

Orders are placed with `Backtest::place_order`; they can be cancelled with
`Backtest::delete_order`. `Backtest::close_position` closes a position at an
explicit price and is applied immediately.

## Account model

| Method | Meaning |
| --- | --- |
| `initial_balance()` | Starting cash. |
| `balance()` | Cash not invested in open positions. |
| `positions_value()` | Mark-to-market value of the open positions. |
| `total_balance()` / `equity()` | `balance() + positions_value()`. This is the account value. |
| `unrealized_pnl()` | Unrealized P&L of the open positions. |
| `fees_paid()` | Cumulative trading fees. |
| `free_balance()` | Cash available for new orders. |

The wallet is re-exported as `bts_rs::engine::Wallet`, so an account view can be
passed around without naming the engine.

`Backtest::new(data, initial_balance, market_fees)` takes fees as **percentages**
(`Some((0.1, 0.05))` means 0.1% for market orders and 0.05% for limit orders).
`Metrics::with_market_fees` takes **fractions** (`Some((0.001, 0.0005))`).
`Metrics::from(&backtest)` converts the rates for you.

## Metrics

With the `metrics` feature, `Metrics::from(&backtest)` reports:

| Metric | Definition |
| --- | --- |
| `pnl()` | Final equity minus initial balance, net of fees. |
| `fees()` | Cumulative fees paid. |
| `max_drawdown()` | Largest peak-to-trough decline of the equity curve, in percent. |
| `profit_factor()` | Gross profits over gross losses on closed trades, net of fees. `0.0` with no trades; `inf` when there are gains and no losses. |
| `win_rate()` | Percentage of closed trades with a positive net result. |
| `sharpe_ratio(rate)` | Annualised mean per-bar equity return over its sample standard deviation. `0.0` when undefined. |

Metrics are computed from the mark-to-market equity curve, not from the cash
balance. See [`docs/EXECUTION-MODEL.md`](https://github.com/raonagos/bts-rs/blob/master/docs/EXECUTION-MODEL.md)
for the exact definitions and the invariants covered by the test suite.

## Feature flags

| Feature | Default | What it enables |
| --- | --- | --- |
| `metrics` | no | `Metrics`, `Event`, and the equity metrics. |
| `optimizer` | no | `Optimizer` and parallel parameter sweeps (pulls in `rayon` and `num_cpus`). |
| `draws` | no | `Draw` and SVG/PNG/HTML charts (pulls in `plotters` and `charming`). |
| `serde` | no | `Serialize`/`Deserialize` for the engine, events, and results. |
| `wasm` | no | `getrandom` and `rayon` support for `wasm32` targets. |

```sh
cargo test --all-features
```

## Examples

```sh
cargo run --example trailing_stop --features metrics,draws
cargo run --example agg_trailing_stop --features metrics,draws
cargo run --example par_parameters_optimization --features optimizer
```

The examples generate deterministic sample candles, so they run without any
external data or network access.

## API stability and MSRV

BTS follows semantic versioning. The public API is documented under
[`docs.rs/bts-rs`](https://docs.rs/bts-rs). The minimum supported Rust version
is **1.88** (the crate uses edition 2024 and let chains).

## Migrating from 1.0.x

Version 1.1 corrects the accounting and the event sampling. If you used 1.0.x:

- Rank and report on `total_balance()` / `equity()`, not on `balance()`. The
  latter is cash and drops when a position is opened.
- Read `pnl()` as `equity - initial_balance`, not as the unrealized
  accumulator.
- `Event::WalletUpdate` now arrives once per bar and carries `equity`; code
  that assumed one event per trade should filter by event type or sample the
  equity curve directly.
- An order placed after seeing a bar fills on the next bar at the earliest.
  Results that relied on same-bar fills will change.
- Fees are charged on the exit notional as well as the entry notional.
- `Metrics::new` takes the final **equity** in its fourth argument.

These are corrections, not cosmetic changes. See [`CHANGELOG.md`](https://github.com/raonagos/bts-rs/blob/master/CHANGELOG.md)
for the full list.

## Limitations

BTS is a bar-level simulator, deliberately:

- No order book, tick tape, or partial fills. A fill is all-or-nothing at the
  order price.
- No slippage, market impact, queue position, latency, or funding model.
- Intrabar order is unknown: when both a take-profit and a stop-loss could
  trigger in the same candle, the take-profit is checked first. Use intrabar
  data or smaller candles if that matters for your strategy.
- No exchange connectivity and no live trading. It never needs API keys.
- No indicator implementations. Use any indicator crate (for example
  [`ta`](https://crates.io/crates/ta)) inside your strategy closure.

## Contributing

Issues and pull requests are welcome. See [`CONTRIBUTING.md`](https://github.com/raonagos/bts-rs/blob/master/CONTRIBUTING.md)
for the local checks and the release process.

## License

MIT. See [`LICENSE`](https://github.com/raonagos/bts-rs/blob/master/LICENSE).
