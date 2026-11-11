# Execution model

This document describes how `bts-rs` turns a sequence of candles into fills,
cash, equity, and metrics. It is the reference for the accounting rules and for
the invariants covered by the test suite. It describes the **current bar-level
implementation**; extensions such as order-book input are listed under
[Limitations](#limitations).

## 1. Event loop

`Backtest::run` consumes one candle at a time, in the order of the input slice:

```text
for each candle c_t:
    1. execute pending orders against c_t's range
    2. evaluate exit rules on positions opened before c_t
    3. mark every open position at c_t's close
    4. record the equity sample for c_t (metrics feature)
    5. call the strategy with c_t
```

An order placed in step 5 is not present during step 1 of `c_t`; it becomes
eligible on `c_{t+1}`. `Backtest::close_position`, by contrast, is an explicit
immediate action: it is applied when the strategy calls it.

The engine does not reorder candles and does not look at future candles.

## 2. Fills

A pending order is filled when its entry price lies inside the candle's
`low..=high` range:

```text
if low <= order.price <= high:
    fill at order.price
```

The fill price is the price carried by the order, not the candle open, close,
or a volume-weighted price. Consequently:

- a market order that is outside the next candle's range is cancelled;
- a limit order remains pending until a candle range contains its price;
- all fills are complete; there are no partial fills;
- there is no slippage, market impact, queue position, or latency model.

Market and limit orders differ in how they are cancelled and which fee rate they
pay, not in the fill rule above.

Exit rules are evaluated on the bars after the position was opened:

| Rule | Long exit | Short exit |
| --- | --- | --- |
| `TakeProfitAndStopLoss(tp, sl)` | `tp` if `tp <= high`, else `sl` if `sl >= low` | `tp` if `tp >= low`, else `sl` if `sl <= high` |
| `TrailingStop(price, percent)` | `price * (1 - percent/100)` if `>= low`; otherwise trail `price` up to `high` | `price * (1 + percent/100)` if `<= high`; otherwise trail `price` down to `low` |

When a take-profit and a stop-loss could both trigger in the same candle, the
take-profit is tested first. A single OHLC bar does not reveal the intrabar
path, so this ordering is a documented convention, not a claim about the true
sequence of trades.

## 3. Account model

```text
cash_at_open   = cash - order.cost
position_value = order.cost + unrealized_pnl
unrealized_pnl = (mark - entry) * quantity       for a long
               = (entry - mark) * quantity       for a short
equity         = cash + sum(position_value)
total_pnl      = equity - initial_balance
```

The accessors make the distinction explicit:

- `Wallet::balance()` is **cash**, the part of the account that is not invested.
- `Wallet::positions_value()` is the mark-to-market value of the open positions.
- `Wallet::total_balance()` and `Wallet::equity()` are `cash + positions_value`.
- `Wallet::unrealized_pnl()` is the sum of the open positions' unrealized P&L.

Opening a position moves cash into `positions_value`; it does not change
equity by itself. Closing a position moves the position's exit value back to
cash and removes it from `positions_value`. Marking is done once per bar, at the
candle close, for every open position.

### Fees

`Backtest::new` accepts `market_fees: Option<(f64, f64)>` as **percentages**.
The engine converts them to fractions internally and charges:

```text
fee = notional * rate
```

on entry and on exit, where `notional = price * quantity`. Market orders use
the first rate and limit orders the second. Fees reduce cash and are
accumulated in `Wallet::fees_paid()`; they are never silently folded into the
fill price.

There is no funding, borrow, or financing charge in the current engine.

## 4. Metrics

With the `metrics` feature, every candle produces one `Event::WalletUpdate`.
The sequence of those events is the equity curve, and all metrics are computed
from it:

| Metric | Definition |
| --- | --- |
| `pnl` | `final_equity - initial_balance`; fees included. |
| `fees` | Cumulative fees paid. |
| `max_drawdown` | Largest peak-to-trough decline of the equity curve, in percent. |
| `profit_factor` | Sum of positive closed-trade P&L over the absolute sum of negative closed-trade P&L, each net of entry and exit fees. `0.0` with no trades; `inf` with gains and no losses. |
| `win_rate` | Percentage of closed trades whose net P&L is strictly positive. |
| `sharpe_ratio` | Mean per-bar equity return minus the per-bar risk-free rate, divided by the sample standard deviation of the returns, multiplied by `sqrt(bars_per_year)`. `0.0` when fewer than two returns exist or the deviation is zero. |

By default the bars-per-year factor is inferred from the median spacing of the
equity samples; `Metrics::with_periods_per_year` overrides it. A trade that
ends exactly at break-even is not a win. An undefined ratio is reported as
`0.0`, never as `NaN`.

The `Display` implementation prints the final cash balance, the final equity,
the P&L, the fees, and the four risk/return metrics.

## 5. Invariants

The test suite asserts, among others:

1. `equity == cash + positions_value` at every marked bar.
2. `pnl == final_equity - initial_balance`, including a position still open at
   the end of the run.
3. Opening a position does not change equity; only fees and mark movements do.
4. An order placed after seeing bar `t` cannot fill on bar `t`.
5. A limit order placed on bar `t` cannot fill on bar `t`'s range.
6. A zero-trade run reports `pnl = 0`, `max_drawdown = 0`, `profit_factor = 0`,
   `win_rate = 0`, and a Sharpe ratio of `0`.
7. `WalletUpdate` is emitted once per bar, not once per trade action.
8. Fees are monotone non-decreasing and are charged on both sides of a trade.

## 6. Aggregated candles

`Backtest::run_with_aggregator` applies the same loop to a base candle and to
aggregated candles built by an `Aggregation` implementation. Aggregated candles
combine open/high/low/close/volume over a fixed number of base candles. The
engine still executes orders against the base candle, so aggregation is a
strategy input, not a second execution path.

## 7. Limitations

- Bar-level input only. There is no tick, trade, or order-book tape.
- No partial fills; a fill is all-or-nothing at the order price.
- No slippage, market impact, queue position, latency, or funding model.
- No short-selling borrow cost and no margin/liquidation model.
- Intrabar ordering is unknown; the take-profit-first rule is a convention.
- No exchange connectivity, credentials, or live order routing.
- No built-in indicators; use an indicator crate inside the strategy closure.

These are explicit trade-offs of a compact, deterministic bar simulator. They
are documented so that a result is never read as more precise than the input
data allows.
