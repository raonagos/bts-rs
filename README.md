<p align="center">
  <img src="logo.png" alt="BTS-RS Logo" width="200"/>
</p>

# bts-rs — execution and accounting for System One

`bts-rs` is the execution and accounting layer of System One. It owns the order
lifecycle, the wallet, the fill model, fees, funding, the per-event equity
curve, and the final balance. It is not an indicator library and it is not the
place where a trading edge is claimed.

> **Status 2026-11-10.** The published `v1.0.11` metrics are defective (M1–M5,
> M7 in the `trad` audit). The fixes are on `fix/metrics-truth`, PR
> [#21](https://github.com/raonagos/bts-rs/pull/21), not yet merged upstream.
> System One pins the fixed commit. `v1.0.11` must not produce a number in this
> project. The event-tape, funding, and ledger pieces in this README are the
> target; the metric fixes are the current code.

## Correctness contract

- **Equity is mark-to-market.** `equity = cash + positions_value`. `balance()`
  is cash and says so; `total_balance()` is equity.
- **P&L is equity minus initial.** It is not the unrealized accumulator.
- **Fills are causal.** An order created after seeing event `t` is first
  eligible at the next event. A limit or stop cannot fill on the range that
  produced the decision.
- **Costs are net.** Profit factor, win rate, and P&L include fees. Funding is
  charged on the held position at each stamp.
- **No hidden liquidity.** Real data has a top-of-book and aggregate depth
  bands, not a full ladder. The default real fill is fill-or-cancel at the best
  quote size; the full-ladder walk is a synthetic-only ablation.
- **One ledger.** `trad` may not keep a second definition of equity or fills.

## What it is not

- Not a source of alpha, indicators, or signals.
- Not a live trading gateway. No keys, no order endpoints, no exchange
  connectivity.
- Not a market-data store. `download-ticks` owns the tape.
- Not a replacement for `trad::balance`’s ceiling. The DP ceiling is `trad`’s;
  this crate executes the position path that the ceiling is compared against.

## Core pieces

| piece | role |
| --- | --- |
| `Candle` | OHLCV + bid/ask + times; the bar-level input |
| `Order` | side, type, size, optional target/reduce-only |
| `Position` | entry, size, side, mark, unrealized P&L |
| `Wallet` | cash, positions value, equity, fees, funding |
| `Backtest` | event loop, order pool, position pool, events |
| `Metrics` | per-event equity metrics: P&L, drawdown, Sharpe, PF, WR |
| `Ledger` (planned) | event-tape accounting API used directly by `trad` |

## Example

```rust
use std::sync::Arc;

use bts_rs::prelude::*;
use chrono::{DateTime, Duration};

let candle = CandleBuilder::builder()
    .open(100.0).high(110.0).low(95.0).close(105.0)
    .volume(1.0).bid(0.5)
    .open_time(DateTime::default())
    .close_time(DateTime::default() + Duration::days(1))
    .build()
    .unwrap();

let mut backtest = Backtest::new(Arc::from_iter(vec![candle]), 10_000.0, None).unwrap();

backtest
    .run(|bt, candle| {
        let order: Order = (OrderType::Market(102.0), 1.0, OrderSide::Buy).into();
        bt.place_order(candle, order)?;
        Ok(())
    })
    .unwrap();

println!("equity: {}", backtest.total_balance());
println!("pnl:    {}", backtest.total_balance() - backtest.initial_balance());
```

The example is bar-level and is kept for API compatibility. System One uses the
event tape and the `Ledger` path; the bar path remains the simplest smoke test.

## Build and test

```sh
cargo build --release
cargo test  --release
```

Feature flags:

| feature | what |
| --- | --- |
| `metrics` | `Metrics` and the equity metrics |
| `optimizer` | parameter sweeps |
| `draws` | SVG/PNG plots |
| `serde` | serialization of events and results |

System One uses `metrics` only. The other features stay optional.

## Roadmap

1. **Merge PR #21** — corrected metrics, next-event fills, net PF/WR.
2. **Event tape input** — accept the canonical `download-ticks` tape instead of
   only a `Vec<Candle>`.
3. **Ledger API** — per-event equity, fees, funding, turnover, final
   liquidation; the API `trad` calls directly.
4. **Real fill model** — top-of-book FOK, conservative limit trade-through,
   no hidden liquidity; depth-band stress model.
5. **Synthetic full-ladder ablation** — walk levels only when the tape actually
   has a ladder, and label the result as an ablation.
6. **Order intents** — open/close long/short, reduce-only, reverse; net
   positions; explicit `hold` at the policy layer.
7. **Parity tests** — same position path as `trad::balance` on the market-order
   path, bit-for-bit where the accounting is the same.

## Migration from `v1.0.11`

Breaking changes already on `fix/metrics-truth`:

- `Metrics::new`’s fourth argument is the final equity, not the unrealized P&L.
- `WalletUpdate` carries `equity`, not cash.
- `Wallet::total_balance()` is equity; `balance()` remains cash.
- `WalletUpdate` is emitted once per event/bar, not once per trade action.
- Orders execute before the strategy sees the bar; a limit cannot fill on the
  bar that placed it.

These are corrections, not cosmetics. A caller that ranked on `balance()` or
read `pnl()` as unrealized was measuring the wrong quantity.

## Documentation

- [`docs/EXECUTION-MODEL.md`](docs/EXECUTION-MODEL.md) — event loop, fills,
  wallet, metrics, invariants.
- [`trad` execution contract](https://github.com/raonagos/trad/blob/main/docs/execution-contract.md)
  — the consumer’s view.
- [`trad` data contract](https://github.com/raonagos/trad/blob/main/docs/data-contract.md)
  — what the tape contains.

## License

MIT. See [`LICENSE`](LICENSE).
