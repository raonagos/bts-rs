# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - 2026-11-11

### Changed

- Metrics are computed from the mark-to-market **equity** curve instead of the
  cash balance. `max_drawdown`, `sharpe_ratio`, `profit_factor`, `win_rate`,
  and `pnl` now describe the account, not its cash sawtooth.
- `Event::WalletUpdate` is emitted once per bar and carries `equity`; it is no
  longer emitted only when a trade happens.
- Orders placed after seeing a bar are first eligible on the next bar. A limit
  order can no longer fill on the bar that placed it.
- The exit fee is charged on the exit notional, not the entry notional.
- `profit_factor` is computed on net trade P&L. It reports `0.0` with no
  trades, `1.0` for all break-even trades, and `inf` only with gains and no
  losses.
- `sharpe_ratio` is annualised from the equity-sample spacing, uses the sample
  standard deviation, and returns `0.0` when undefined instead of `NaN`.
- The optimizer example ranks parameter sets on equity rather than cash.
- `Candle` documents `bid`/`ask` as bid-side and ask-side **volumes**, and adds
  `bid_volume()` and `ask_volume()` aliases. `CandleBuilder` rejects a bid-side
  volume outside `0..=volume`.
- `Wallet` is publicly re-exported, so it can be named and used as a
  read-only account view (constructors and mutators stay crate-private).
- `Error::InvalideTimes` is renamed to `Error::InvalidTimes`; the old spelling
  remains as a deprecated variant for source compatibility.
- `delete_order` and `close_position` always remove the order or position from
  the pool. The `force_remove` argument is retained for source compatibility.
- Closing a position rebuilds the aggregate book from the positions still open,
  so equity is correct even when the exit price differs from the last mark.
- The drawing backend now propagates drawing errors instead of panicking, and
  volume bars use the candle duration instead of a fixed day.
- `OrderSide` and `PositionSide` implement `PartialEq`, `Eq`, and `Hash`.
- `Metrics` implements `Debug` and `Clone`; `CandleBuilder` implements
  `Default`.
- `Event::from` no longer panics when the wallet's free balance is negative.

### Fixed

- Removed a memory leak when deserializing `Error::MissingField` with the
  `serde` feature.
- Fixed the broken intra-doc links in the metrics module.
- Fixed the clippy `for_kv_map` lint in `run_with_aggregator`.
- Fixed equity drift when a position was closed at a price different from the
  last mark (for example, with `close_all_positions`).

### Documentation

- Rewrote the README as a general-purpose library guide: installation,
  quickstart, engine model, account model, metrics, features, examples, MSRV,
  and limitations.
- Rewrote `docs/EXECUTION-MODEL.md` as a general reference for the execution
  and accounting model; removed project-specific references.
- Rewrote `CONTRIBUTING.md` and documented the release process.
- Added a CI workflow with formatting, clippy, tests, documentation, a feature
  matrix, a wasm build, and an MSRV check.
- Declared the MSRV explicitly: Rust 1.88 (edition 2024 plus let chains).
- Reworked the crates.io release workflow to validate the tag against
  `Cargo.toml`, run the tests, and publish with `--locked`.

### Removed

- Removed internal-project framing from the README and the execution-model
  document. `bts-rs` is an independent library.

## [1.0.11] - 2025-11-09

Previous published release. The metrics in this version were computed from the
cash balance and are superseded by 1.1.0.

[1.1.0]: https://github.com/raonagos/bts-rs/compare/v1.0.11...v1.1.0
[1.0.11]: https://github.com/raonagos/bts-rs/releases/tag/v1.0.11
