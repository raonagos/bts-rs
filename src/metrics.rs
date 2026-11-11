//! Performance metrics for backtesting.
//!
//! This module provides tools to calculate:
//! - Max drawdown
//! - Profit factor
//! - Sharpe ratio (annualised)
//! - Win rate
//!
//! Events generated during backtesting.
//!
//! This module defines the `Event` enum, which represents actions and state changes
//! during a backtest, such as order execution, position updates, and wallet changes.
//!
//! It needs to enable `metrics` feature to use it. Take a look at [trailing stop](https://github.com/raonagos/bts-rs/blob/master/examples/trailing_stop.rs#L62) for example.
//!
//! # What the numbers are
//!
//! Every metric is computed from the **mark-to-market equity** of the account
//! (`cash + cost + unrealized pnl` of the open positions), sampled once per bar by
//! [`Backtest::run`](crate::engine::Backtest::run) as an [`Event::WalletUpdate`].
//!
//! - `max_drawdown` is the largest peak-to-trough decline of that equity curve, in
//!   percent. It is not the position size, and it does not depend on how large the
//!   notional is relative to the account.
//! - `sharpe_ratio` is the mean of the per-bar equity returns over their sample
//!   standard deviation, scaled by `sqrt(bars per year)`. The bars per year are
//!   inferred from the event timestamps (and can be overridden), so the number is
//!   comparable to a published Sharpe.
//! - `profit_factor` and `win_rate` are computed on the **net** profit and loss of
//!   each closed trade: the gross `(exit - entry) * quantity` minus the market or
//!   limit fee charged on both sides.

use std::fmt;

use crate::engine::*;

use chrono::{DateTime, Utc};

/// Seconds in a Julian year, used to annualise a per-bar Sharpe ratio.
const SECONDS_PER_YEAR: f64 = 365.25 * 24.0 * 3600.0;

/// Events generated during a backtest.
///
/// Each event corresponds to an action or state change, such as:
/// - Adding or removing orders/positions.
/// - Updating the wallet balance.
/// - Charging fees.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// An order has been added to the backtest.
    ///
    /// This event is triggered when a new order is created and added to the order queue.
    AddOrder(DateTime<Utc>, Order),

    /// An order has been removed from the backtest.
    ///
    /// This event is triggered when an order is canceled or executed.
    DelOrder(DateTime<Utc>, Order),

    /// A position has been opened.
    ///
    /// This event is triggered when an order is executed and a new position is created.
    AddPosition(DateTime<Utc>, Position),

    /// A position has been closed.
    ///
    /// This event is triggered when a position is closed, either manually or by an exit rule.
    /// The position carries its exit price, so `position.pnl()` is the gross result of the
    /// trade; the fees charged on both sides are not in it.
    DelPosition(DateTime<Utc>, Position),

    /// The wallet has been marked to market.
    ///
    /// This event is emitted **once per bar**, after the bar's orders and exits have been
    /// executed and before the strategy sees the bar, so the series of these events is the
    /// equity curve the metrics are computed from.
    WalletUpdate {
        /// Moment
        datetime: DateTime<Utc>,
        /// Cumulative net profit and loss: `equity - initial_balance`.
        pnl: f64,
        /// Total fees paid.
        fees: f64,
        /// Available funds (not locked in open positions).
        free: f64,
        /// Funds locked in open positions.
        locked: f64,
        /// Cash: the balance that is not invested in open positions.
        balance: f64,
        /// Mark-to-market account value: `balance + market value of the open positions`.
        equity: f64,
    },
}

impl From<(DateTime<Utc>, &Wallet)> for Event {
    fn from((datetime, value): (DateTime<Utc>, &Wallet)) -> Self {
        Self::WalletUpdate {
            datetime,
            locked: value.locked(),
            fees: value.fees_paid(),
            balance: value.balance(),
            equity: value.total_balance(),
            pnl: value.total_balance() - value.initial_balance(),
            free: value.free_balance().expect("should give the free balance"),
        }
    }
}

/// A collection of trading metrics calculated from a series of events.
///
/// `Metrics` is used to compute and display key performance indicators (KPIs)
/// for a trading strategy, such as max drawdown, profit factor, Sharpe ratio, and win rate.
/// It is typically constructed from a `Backtest` or a list of `Event`s.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Metrics {
    fees: f64,
    balance: f64,
    equity: f64,
    events: Vec<Event>,
    initial_balance: f64,
    market_fees: Option<(f64, f64)>,
    periods_per_year: Option<f64>,
}

impl From<&Backtest> for Metrics {
    fn from(value: &Backtest) -> Self {
        Self {
            fees: value.fees_paid(),
            balance: value.balance(),
            equity: value.total_balance(),
            initial_balance: value.initial_balance(),
            events: value.events().cloned().collect(),
            market_fees: value.market_fees().copied(),
            periods_per_year: None,
        }
    }
}

impl Metrics {
    /// Creates a new `Metrics` instance from a list of events, an initial balance,
    /// the final cash balance, the final equity, and the cumulative fees paid.
    ///
    /// Trade fees are unknown here, so [`profit_factor`](Self::profit_factor) and
    /// [`win_rate`](Self::win_rate) are gross unless [`with_market_fees`](Self::with_market_fees)
    /// is called with the backtest's fee rates. [`Metrics::from(&Backtest)`] does that for you.
    pub fn new(events: Vec<Event>, initial_balance: f64, balance: f64, equity: f64, fees: f64) -> Self {
        Self {
            fees,
            events,
            balance,
            equity,
            initial_balance,
            market_fees: None,
            periods_per_year: None,
        }
    }

    /// Sets the fee rates used to net the per-trade profit and loss.
    ///
    /// The tuple is `(market, limit)` as a **fraction** (`0.001` for 0.1%), the same
    /// convention as [`Backtest::market_fees`](crate::engine::Backtest::market_fees).
    pub fn with_market_fees(mut self, market_fees: Option<(f64, f64)>) -> Self {
        self.market_fees = market_fees;
        self
    }

    /// Overrides the number of bars per year used to annualise the Sharpe ratio.
    ///
    /// By default it is inferred from the spacing of the equity samples.
    pub fn with_periods_per_year(mut self, periods_per_year: f64) -> Self {
        self.periods_per_year = Some(periods_per_year);
        self
    }

    /// Returns the initial balance.
    pub fn initial_balance(&self) -> f64 {
        self.initial_balance
    }

    /// Returns the events.
    pub fn events(&self) -> std::slice::Iter<'_, Event> {
        self.events.iter()
    }

    /// Returns the final cash balance: the part of the account that is not in open positions.
    ///
    /// Rank a strategy on [`equity()`](Self::equity), not on this.
    pub fn balance(&self) -> f64 {
        self.balance
    }

    /// Returns the final mark-to-market equity of the account.
    pub fn equity(&self) -> f64 {
        self.equity
    }

    /// Returns the cumulative fees paid.
    pub fn fees(&self) -> f64 {
        self.fees
    }

    /// Returns the net profit and loss: `equity - initial_balance`, fees included.
    pub fn pnl(&self) -> f64 {
        self.equity - self.initial_balance
    }

    /// Returns the equity curve, one sample per bar, starting from the initial balance.
    fn equity_series(&self) -> Vec<f64> {
        let mut series = Vec::with_capacity(self.events.len() + 1);
        series.push(self.initial_balance);
        for event in &self.events {
            if let Event::WalletUpdate { equity, .. } = event {
                series.push(*equity);
            }
        }
        series
    }

    /// Computes the maximum drawdown of the equity curve, as a percentage.
    ///
    /// It is the largest peak-to-trough decline of the mark-to-market account value,
    /// so a position that never loses money reports `0.0` whatever its notional is.
    pub fn max_drawdown(&self) -> f64 {
        let series = self.equity_series();

        let mut peak = self.initial_balance;
        let mut max_drawdown = 0.0;

        for equity in series.iter().skip(1) {
            if *equity > peak {
                peak = *equity;
            }
            if peak > 0.0 {
                let drawdown = (peak - equity) / peak;
                if drawdown > max_drawdown {
                    max_drawdown = drawdown;
                }
            }
        }

        max_drawdown * 100.0
    }

    /// Returns the net profit and loss of one closed trade, fees included.
    ///
    /// Both sides are charged the fee rate of the position's entry type, on the notional
    /// of the position: `gross pnl - 2 * cost * rate`.
    fn net_trade_pnl(&self, position: &Position) -> f64 {
        let gross = position.pnl().unwrap_or(0.0);
        let cost = position.cost().unwrap_or(0.0);
        let rate = match self.market_fees {
            Some((market_fee, limit_fee)) => {
                if position.is_market_type() {
                    market_fee
                } else {
                    limit_fee
                }
            }
            None => 0.0,
        };
        gross - cost * rate * 2.0
    }

    /// Returns the net result of every closed trade.
    fn trade_results(&self) -> Vec<f64> {
        self.events
            .iter()
            .filter_map(|event| match event {
                Event::DelPosition(_, position) => Some(self.net_trade_pnl(position)),
                _ => None,
            })
            .collect()
    }

    /// Computes the profit factor: gross profits over gross losses, both net of fees.
    ///
    /// - no closed trade: `0.0`;
    /// - trades that are all exactly break-even after fees: `1.0`;
    /// - gains and no losses: `f64::INFINITY`.
    pub fn profit_factor(&self) -> f64 {
        let results = self.trade_results();
        if results.is_empty() {
            return 0.0;
        }

        let mut gains = 0.0;
        let mut losses = 0.0;
        for pnl in results {
            if pnl > 0.0 {
                gains += pnl;
            } else {
                losses -= pnl;
            }
        }

        if losses == 0.0 {
            return if gains > 0.0 { f64::INFINITY } else { 1.0 };
        }

        gains / losses
    }

    /// Returns the number of bars per year, inferred from the spacing of the equity samples.
    fn inferred_periods_per_year(&self) -> f64 {
        let mut deltas: Vec<i64> = Vec::new();
        let mut previous: Option<i64> = None;
        for event in &self.events {
            if let Event::WalletUpdate { datetime, .. } = event {
                let stamp = datetime.timestamp();
                if let Some(previous) = previous
                    && stamp > previous
                {
                    deltas.push(stamp - previous);
                }
                previous = Some(stamp);
            }
        }

        if deltas.is_empty() {
            return 1.0;
        }

        deltas.sort_unstable();
        let median = deltas[deltas.len() / 2] as f64;
        let periods_per_year = SECONDS_PER_YEAR / median;
        if periods_per_year.is_finite() && periods_per_year >= 1.0 {
            periods_per_year
        } else {
            1.0
        }
    }

    /// Computes the Sharpe ratio of the per-bar equity returns.
    ///
    /// `risk_free_rate` is the **annualised** risk-free return (`0.0` for none); it is
    /// divided by the number of bars per year before being subtracted from the mean bar
    /// return. The result is annualised with `sqrt(bars per year)`, on the sample standard
    /// deviation (`n - 1`).
    ///
    /// Fewer than two bars, or a constant curve, returns `0.0`: an undefined ratio is
    /// reported as no risk-adjusted return rather than as `NaN`.
    pub fn sharpe_ratio(&self, risk_free_rate: f64) -> f64 {
        let periods_per_year = self
            .periods_per_year
            .unwrap_or_else(|| self.inferred_periods_per_year());
        self.sharpe_ratio_with(risk_free_rate, periods_per_year)
    }

    /// Computes the anualised Sharpe ratio with an explicit number of bars per year.
    pub fn sharpe_ratio_with(&self, risk_free_rate: f64, periods_per_year: f64) -> f64 {
        if !(periods_per_year.is_finite() && periods_per_year > 0.0) {
            return 0.0;
        }

        let series = self.equity_series();
        let mut returns = Vec::with_capacity(series.len());
        let mut previous = self.initial_balance;
        for equity in series.iter().skip(1) {
            if previous > 0.0 && equity.is_finite() {
                returns.push(equity / previous - 1.0);
            }
            previous = *equity;
        }

        let n = returns.len();
        if n < 2 {
            return 0.0;
        }

        let mean = returns.iter().sum::<f64>() / n as f64;
        let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
        let std_dev = variance.sqrt();
        if !std_dev.is_finite() || std_dev == 0.0 {
            return 0.0;
        }

        let sharpe = (mean - risk_free_rate / periods_per_year) / std_dev * periods_per_year.sqrt();
        if sharpe.is_finite() { sharpe } else { 0.0 }
    }

    /// Computes the win rate as a percentage of profitable closed trades, net of fees.
    ///
    /// A trade that ends exactly at break-even is not a win.
    pub fn win_rate(&self) -> f64 {
        let results = self.trade_results();
        if results.is_empty() {
            return 0.0;
        }

        let winners = results.iter().filter(|pnl| **pnl > 0.0).count();
        (winners as f64 / results.len() as f64) * 100.0
    }
}

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== Backtest Metrics ===")?;
        writeln!(f, "Initial Balance: {:.2}", self.initial_balance)?;
        writeln!(f, "Final Balance: {:.2}", self.balance)?;
        writeln!(f, "Final Equity: {:.2}", self.equity)?;
        writeln!(f, "Profit & Loss (P&L): {:.2}", self.pnl())?;
        writeln!(f, "Fees paid: {:.2}", self.fees)?;
        #[allow(clippy::writeln_empty_string)]
        writeln!(f, "")?;
        writeln!(f, "Max Drawdown: {:.2}%", self.max_drawdown())?;
        writeln!(f, "Profit Factor: {:.2}", self.profit_factor())?;
        writeln!(
            f,
            "Sharpe Ratio (annualised, risk-free rate = 0.0): {:.2}",
            self.sharpe_ratio(0.0)
        )?;
        writeln!(f, "Win Rate: {:.2}%", self.win_rate())
    }
}

#[cfg(test)]
// Helper function to create a simple position for testing
fn create_position(pnl: f64) -> Position {
    let order: Order = (OrderType::Market(100.0), 1.0, OrderSide::Buy).into();
    let mut position = Position::from(order);
    // Mock the pnl() method for testing
    // In a real scenario, you would set the exit price or mock the behavior
    // Here, we assume Position has a method to set pnl directly for testing
    position.set_exit_price(100.0 + pnl).unwrap(); // Simulate a P&L of `pnl`
    position
}

#[cfg(test)]
fn update(balance: f64, equity: f64) -> Event {
    Event::WalletUpdate {
        datetime: DateTime::default(),
        pnl: 0.0,
        fees: 0.0,
        free: balance,
        locked: 0.0,
        balance,
        equity,
    }
}

#[cfg(test)]
#[test]
fn max_drawdown() {
    let events = vec![
        update(10000.0, 10000.0),
        update(12000.0, 12000.0),
        update(9000.0, 9000.0),
        update(11000.0, 11000.0),
    ];
    let metrics = Metrics::new(events, 10000.0, 0.0, 11000.0, 0.0);
    assert_eq!(metrics.max_drawdown(), 25.0); // (12000 - 9000) / 12000 = 25%
}

#[cfg(test)]
#[test]
fn max_drawdown_ignores_cash_sawtooth() {
    // A 90% notional position: the cash swings by the whole notional at each
    // open and close, the equity does not move at all.
    let events = vec![
        update(100.0, 1000.0),  // opened at 100
        update(100.0, 1090.0),  // marked at 110
        update(1090.0, 1090.0), // closed at 110
    ];
    let metrics = Metrics::new(events, 1000.0, 1090.0, 1090.0, 0.0);
    assert_eq!(metrics.max_drawdown(), 0.0);
    assert_eq!(metrics.pnl(), 90.0);

    let events = vec![
        update(100.0, 1000.0),
        update(100.0, 550.0), // 90% notional, exit at 50
        update(550.0, 550.0),
    ];
    let metrics = Metrics::new(events, 1000.0, 550.0, 550.0, 0.0);
    assert_eq!(metrics.max_drawdown(), 45.0); // not 90.0
    assert_eq!(metrics.pnl(), -450.0);
}

#[cfg(test)]
#[test]
fn max_drawdown_no_events() {
    let metrics = Metrics::new(vec![], 10000.0, 0.0, 0.0, 0.0);
    assert_eq!(metrics.max_drawdown(), 0.0); // No drawdown if no events
}

#[cfg(test)]
#[test]
fn profit_factor() {
    let winning_position = create_position(20.0);
    let losing_position = create_position(-10.0);
    let events = vec![
        Event::DelPosition(DateTime::default(), winning_position),
        Event::DelPosition(DateTime::default(), losing_position),
    ];
    let metrics = Metrics::new(events, 10000.0, 0.0, 0.0, 0.0);
    assert_eq!(metrics.profit_factor(), 2.0); // 20 / 10 = 2.0
}

#[cfg(test)]
#[test]
fn profit_factor_net_of_fees() {
    // the trade gains 1.0 and pays 0.1% of a 100 notional on both sides = 0.2
    let mut position = create_position(1.0);
    position.set_exit_price(101.0).unwrap();
    let events = vec![Event::DelPosition(DateTime::default(), position)];
    let metrics = Metrics::new(events, 10000.0, 0.0, 0.0, 0.0).with_market_fees(Some((0.001, 0.001)));
    assert_eq!(metrics.win_rate(), 100.0);
    assert!((metrics.net_trade_pnl(&position) - 0.8).abs() < 1e-12);
}

#[cfg(test)]
#[test]
fn profit_factor_break_even_after_fees() {
    // a flat trade that pays fees is a loss, not an infinity
    let mut position = create_position(0.0);
    position.set_exit_price(100.0).unwrap();
    let events = vec![Event::DelPosition(DateTime::default(), position)];
    let metrics = Metrics::new(events, 10000.0, 0.0, 0.0, 0.0).with_market_fees(Some((0.001, 0.001)));
    assert_eq!(metrics.profit_factor(), 0.0);
    assert_eq!(metrics.win_rate(), 0.0);

    // with no fee rate to net, the same trade is exactly break-even
    let metrics = Metrics::new(
        vec![Event::DelPosition(DateTime::default(), create_position(0.0))],
        10000.0,
        0.0,
        0.0,
        0.0,
    );
    assert_eq!(metrics.profit_factor(), 1.0);
}

#[cfg(test)]
#[test]
fn profit_factor_no_losses() {
    let winning_position = create_position(20.0);
    let events = vec![Event::DelPosition(DateTime::default(), winning_position)];
    let metrics = Metrics::new(events, 10000.0, 0.0, 0.0, 0.0);
    assert_eq!(metrics.profit_factor(), f64::INFINITY); // No losses
}

#[cfg(test)]
#[test]
fn profit_factor_no_trades() {
    let metrics = Metrics::new(vec![], 10000.0, 0.0, 0.0, 0.0);
    assert_eq!(metrics.profit_factor(), 0.0); // No trades: no factor, not an infinity
}

#[cfg(test)]
#[test]
fn sharpe_ratio() {
    let events = vec![
        update(10000.0, 10000.0),
        update(10500.0, 10500.0),
        update(10300.0, 10300.0),
        update(10700.0, 10700.0),
    ];
    let metrics = Metrics::new(events, 10000.0, 0.0, 10700.0, 0.0);
    let sharpe = metrics.sharpe_ratio(0.0);
    // per bar: +5%, -1.90%, +3.88%; positive, finite
    assert!(sharpe > 0.0 && sharpe.is_finite());
}

#[cfg(test)]
#[test]
fn sharpe_ratio_no_events() {
    let metrics = Metrics::new(vec![], 10000.0, 0.0, 0.0, 0.0);
    // undefined, reported as no risk-adjusted return instead of NaN
    assert_eq!(metrics.sharpe_ratio(0.0), 0.0);
}

#[cfg(test)]
#[test]
fn sharpe_ratio_is_sized_and_annualised() {
    // 20 identical +1% bars: the same bar return repeated has zero dispersion,
    // so the ratio is 0 whatever the notional is.
    let mut events = Vec::new();
    let mut equity = 1000.0;
    for _ in 0..20 {
        equity *= 1.01;
        events.push(update(equity, equity));
    }
    let metrics = Metrics::new(events, 1000.0, equity, equity, 0.0);
    assert_eq!(metrics.sharpe_ratio(0.0), 0.0);

    // one losing bar in twenty: the mean stays positive, the deviation does not
    let mut events = Vec::new();
    let mut equity = 1000.0;
    for i in 0..20 {
        equity *= if i == 7 { 0.995 } else { 1.01 };
        events.push(update(equity, equity));
    }
    let flat = Metrics::new(events.clone(), 1000.0, equity, equity, 0.0);
    let scaled = Metrics::new(events, 1000.0, equity, equity, 0.0).with_periods_per_year(8760.0);
    let (flat, scaled) = (flat.sharpe_ratio_with(0.0, 1.0), scaled.sharpe_ratio(0.0));
    assert!(flat > 0.0);
    // annualising 8760 bars a year scales the ratio by sqrt(8760), not by the notional
    assert!((scaled / flat - 8760.0_f64.sqrt()).abs() < 1e-6);
}

#[cfg(test)]
#[test]
fn win_rate() {
    let winning_position = create_position(20.0);
    let losing_position = create_position(-10.0);
    let events = vec![
        Event::DelPosition(DateTime::default(), winning_position),
        Event::DelPosition(DateTime::default(), losing_position),
    ];
    let metrics = Metrics::new(events, 10000.0, 0.0, 0.0, 0.0);
    assert_eq!(metrics.win_rate(), 50.0); // 1 win out of 2 trades
}

#[cfg(test)]
#[test]
fn win_rate_no_trades() {
    let metrics = Metrics::new(vec![], 10000.0, 0.0, 0.0, 0.0);
    assert_eq!(metrics.win_rate(), 0.0); // No trades
}

#[cfg(test)]
#[test]
fn win_rate_all_winning() {
    let winning_position = create_position(20.0);
    let events = vec![Event::DelPosition(DateTime::default(), winning_position)];
    let metrics = Metrics::new(events, 10000.0, 0.0, 0.0, 0.0);
    assert_eq!(metrics.win_rate(), 100.0); // 1 win out of 1 trade
}

#[cfg(test)]
#[test]
fn pnl_is_the_equity_change() {
    // cash 100, a 990 position open, equity 1090: the P&L is +90, not 0
    let events = vec![update(100.0, 1090.0)];
    let metrics = Metrics::new(events, 1000.0, 100.0, 1090.0, 0.0);
    assert_eq!(metrics.balance(), 100.0);
    assert_eq!(metrics.equity(), 1090.0);
    assert_eq!(metrics.pnl(), 90.0);
}
