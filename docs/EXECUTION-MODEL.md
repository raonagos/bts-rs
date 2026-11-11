# Execution model

**Status:** target model for the System One integration, 2026-11-10.
**Code:** corrected metrics are on `fix/metrics-truth` (PR #21); the event-tape
and L2/ledger pieces are planned.

This document is the correctness contract for `bts-rs` as the execution layer
of System One. It describes what a fill is, what equity is, and which tests
must stay green.

## 1. Event loop

The engine consumes one event at a time, in exchange-time order:

```text
1. receive e_t
2. mark open positions at e_t's mark price
3. charge funding if e_t is a funding stamp
4. execute orders that became eligible at e_t
5. emit the per-event equity mark
6. ask the strategy for a decision at e_t
7. enqueue the resulting orders for the next eligible event
```

A strategy sees the event after the fills from that event are applied. An order
created at step 6 is **not** eligible at `e_t`; it becomes eligible at the next
event. This is the no-lookahead invariant.

For a bar-level caller, one candle-close event is one event. The same rule
applies: a market order decided on the close fills at the next event, not at
the close that produced the decision.

## 2. Mark price

The mark is the price used to value open positions:

- if a trade occurred at the event, the last trade price;
- else the mid of the top of book;
- else the previous mark, with a stale flag.

The mark is for equity, not for fills. A fill uses the eligible event's
executable price (best ask for a buy, best bid for a sell). The two are allowed
to differ; the difference is the spread/slippage, reported separately.

## 3. Orders

### 3.1 Types

| type | status | fill rule |
| --- | --- | --- |
| market | required | best bid/ask for the visible size; FOK beyond it on real data |
| limit | research | trade-through only; no fill on the placing event or its range |
| stop / take-profit | not in v1 | explicit policy action or pre-registered price rule |

A limit order may not fill on the event that placed it, and may not fill on the
range of that event. A stop placed after reading a high/low may not fill on the
same bar.

### 3.2 Intent

```text
side        Buy | Sell
intent      Open | Close | Reduce | Reverse
size        signed target delta, in equity units
position_id optional; reduce-only when set
```

Positions are net. A reverse is one net order whose delta is
`target - current`; it may be filled as one net trade or two child fills, but
the fee is charged on the full `|delta|`.

### 3.3 Real versus synthetic fills

Real historical data has no full ladder:

- `bookTicker`: one level per side per update;
- `bookDepth`: aggregate depth at ±1..5% every ~30 s;
- `aggTrades`: event-level trades.

The default real fill is therefore:

```text
buy:  fill at ask if size <= ask_size, else no fill (FOK)
sell: fill at bid if size <= bid_size, else no fill (FOK)
fee  = fee_bps * notional
```

No hidden liquidity. No extrapolated ladder. A policy that wants a larger
position must slice across events, and the slicing rule is reported.

Synthetic data may carry a full ladder. A full-ladder walk is allowed only when
the tape actually has one, and the result is labelled a **synthetic ablation**.
It is never presented as a real-data result.

A depth-band stress model may be used for robustness:

```text
impact_bps = spread_bps/2 + k * size / depth_at_1pct
```

`k` is fixed before the run. The stress result is reported beside the default,
not instead of it.

## 4. Wallet and equity

```text
cash          = initial + realised_pnl - fees - funding
positions     = sum(mark_value(position))
unrealised    = sum((mark - entry) * size * side)
equity        = cash + positions
```

- `balance()` is cash. It moves when a position opens; it is not account value.
- `total_balance()` / `equity()` is `cash + positions_value`; this is the only
  number used by metrics and by `trad`.
- At the end of a run, open positions are liquidated at the last mark and pay
  their exit fee. `final_equity` is after that liquidation.
- Funding is charged to cash at each stamp: a long pays a positive rate, a
  short receives it. The signed series is reported.

## 5. Metrics

Computed from the per-event equity curve, net of fees and funding:

| metric | definition |
| --- | --- |
| `pnl` | `final_equity - initial_balance` |
| `fees` | cumulative fees paid |
| `funding` | cumulative funding paid (positive = cost) |
| `max_drawdown` | worst peak-to-trough of the equity curve |
| `profit_factor` | gross profit / gross loss on net trade P&L; 0 with no trades, `inf` only with gains and no losses |
| `win_rate` | winning closed trades / closed trades, net of fees |
| `sharpe` | per-event or per-bar equity returns, sample std, annualised with an explicit periods-per-year; 0 when undefined |

Sharpe is a diagnostic. System One scores on final balance, `efficiency`,
`capture`, and `gap` against the `trad` DP ceiling.

## 6. Invariants

Tests, not comments:

1. `equity == cash + positions_value` at every event.
2. `pnl == final_equity - initial_balance`, including an open final position.
3. Fees are monotone non-decreasing. Funding is signed and reported.
4. A market order placed at `t` cannot fill at `t`.
5. A limit placed at `t` cannot fill at `t` or on `t`'s range.
6. A real fill price is the eligible event’s best bid/ask and its size does not
   exceed the visible quote (FOK).
7. A synthetic ladder fill never exceeds the visible ladder.
8. A zero-trade run has Sharpe 0, PF 0, WR 0, drawdown 0.
9. A flat run has `pnl = 0`, `fees = 0`, `final = initial`.
10. The same position path with market fills gives the same final balance as
    `trad::balance::balance` on the same prices, fees, and funding.

## 7. Events

The corrected `Event::WalletUpdate` is emitted **once per event**, at the mark,
and carries:

```text
datetime, pnl, fees, free, locked, balance(equity)
```

It is not emitted only on trade actions. A per-trade sample sees only the bars
where something happened, so an intra-trade excursion is invisible and the open
notional reads as a realised loss.

## 8. Integration with `trad`

`trad` uses two paths:

- **Fast path (DP labels and bulk scoring):** `balance::balance` and
  `balance::max_balance` on a position path. Exact on the grid.
- **Execution path (policy rollout):** `bts-rs`, event by event, with the same
  fees, funding, and final liquidation.

A parity test asserts the two paths agree on a market-order-only path. If they
disagree, the execution contract is the source of truth for the fill and the DP
is updated to match, or the discrepancy is reported as a bug.

## 9. Non-goals

- No exchange connectivity, keys, or live orders.
- No hidden liquidity, queue-position optimism, or maker rebate assumptions.
- No indicator library and no clock feature in the execution layer.
- No “fill at the close the signal read” in the new path. The old close-fill
  convention survives only in the archived `trad` numbers.
