//! # BTS: BackTest Strategy for Trading Algorithms
//!
//! **BTS** is a high-performance Rust library for backtesting trading strategies on candlestick (OHLCV) data.
//! It is designed for **speed, flexibility, and accuracy**, making it ideal for both **retail traders** and **algorithmic trading developers**.
//!
//! ## Why BTS?
//! - **Optimized for Performance**: Uses `VecDeque` for O(1) operations on orders/positions, and parallel processing for optimization tasks.
//! - **Technical Analysis Ready**: Seamlessly integrates with the [`ta`](https://crates.io/crates/ta) crate for 100+ indicators (EMA, MACD, RSI, etc.).
//! - **Risk Management**: Supports stop-loss, take-profit, and trailing stops with configurable rules.
//! - **Realistic Simulations**: Models slippage, fees, and latency for accurate backtesting.
//! - **Extensible**: Add custom indicators, strategies, or data sources with minimal effort.
//!
//! ## Core Components
//! | Component   | Description                                                                                     |
//! |-------------|-------------------------------------------------------------------------------------------------|
//! | **`Candle`** | Represents OHLCV (Open, High, Low, Close, Volume) data for a single time period.               |
//! | **`Order`**  | Market, limit, or conditional orders (e.g., stop-loss, take-profit).                          |
//! | **`Position`** | Open trades with configurable exit rules (e.g., trailing stops).                              |
//! | **`Wallet`** | Tracks balance, locked funds, unrealized P&L, and fees.                                       |
//! | **`Metrics`** | Calculates performance metrics: P&L, drawdown, Sharpe ratio, win rate, and more.             |
//! | **`Backtest`** | The engine that simulates strategy execution over historical data.                          |
//!
//! ## Features
//! ### 1. **Technical Indicators**
//! - Built-in support for **Volume Profile**, **Impulse MACD**, and **custom indicators**.
//! - Compatible with the [`ta`](https://crates.io/crates/ta) crate for 100+ additional indicators.
//!
//! ### 2. **Order Types & Exit Rules**
//! | Order Type               | Description                                                                                     |
//! |--------------------------|-------------------------------------------------------------------------------------------------|
//! | **Market Order**         | Executes immediately at the current price.                                                    |
//! | **Limit Order**          | Executes only at a specified price or better.                                                 |
//! | **Take-Profit**          | Closes the position when a target price is reached.                                          |
//! | **Stop-Loss**            | Closes the position to limit losses.                                                          |
//! | **Trailing Stop**        | Dynamically adjusts the stop price based on market movements.                                |
//! | **Take-Profit + Stop-Loss** | Combines both rules for risk management.                                                   |
//!
//! ### 3. **Performance Metrics**
//! | Metric               | Description                                                                                     |
//! |----------------------|-------------------------------------------------------------------------------------------------|
//! | **Max Drawdown**     | Largest peak-to-trough decline in account balance (%).                                        |
//! | **Profit Factor**    | Ratio of gross profits to gross losses.                                                       |
//! | **Sharpe Ratio**     | Risk-adjusted return (higher = better).                                                      |
//! | **Win Rate**         | Percentage of winning trades.                                                                 |
//! | **Sortino Ratio**    | Like Sharpe ratio, but focuses only on downside volatility.                                  |
//!
//! ### 4. **Optimization Tools**
//! - **Parallel Brute-Force**: Optimize strategy parameters (e.g., EMA periods) using multi-threading.
//!
//! ## Getting Started
//! ### 1. Add BTS to your project:
//! ```toml
//! [dependencies]
//! bts = "*"
//! ta = "0.5"  # Optional: For technical analysis indicators
//! ```
//!
//! ### 2. Run a Simple Backtest:
//! ```rust
//! use bts::prelude::*;
//! use chrono::DateTime;
//!
//! fn main() {
//!     let candle = CandleBuilder::builder()
//!         .open(100.0)
//!         .high(110.0)
//!         .low(95.0)
//!         .close(105.0)
//!         .volume(1.0)
//!         .bid(0.5)
//!         .open_time(DateTime::default())
//!         .close_time(DateTime::default())
//!         .build()
//!         .unwrap();
//!
//!     // Initialize backtest with \$10,000
//!     let mut backtest = Backtest::new(vec![candle], 10_000.0, None).unwrap();
//!
//!     // Execute a market buy order
//!     backtest
//!         .run(|bt, _candle| {
//!             let order: Order = (OrderType::Market(102.0), 1.0, OrderSide::Buy).into();
//!             bt.place_order(order).unwrap();
//!
//!             // Close the position at \$104.0
//!             if let Some(position) = bt.positions().last().cloned() {
//!                 bt.close_position(&position, 104.0, true).unwrap();
//!             }
//!
//!             Ok(())
//!         })
//!         .unwrap();
//!
//!     // Print performance metrics
//!     #[cfg(feature = "metrics")]
//!     {
//!         let metrics = Metrics::from(&backtest);
//!         println!("{}", metrics);
//!     }
//! }
//! ```
//!
//! ### Output:
//! ```bash
//! === Backtest Metrics ===
//! Initial Balance: 10000.00
//! Final Balance: 10018.00
//! Max Drawdown: 0.20%
//! Profit Factor: 2.00
//! Sharpe Ratio: 1.50
//! Win Rate: 100.00%
//! ```
//!
//! ## Use Cases
//! - **Retail Traders**: Test manual strategies before risking real capital.
//! - **Algo Developers**: Build and optimize automated trading systems.
//! - **Quant Researchers**: Backtest statistical arbitrage or machine learning models.
//! - **Educational**: Teach trading concepts with a hands-on tool.
//!
//! ## Integrations
//! | Crate          | Purpose                                                                                     |
//! |----------------|---------------------------------------------------------------------------------------------|
//! | [`ta`](https://crates.io/crates/ta) | Technical analysis indicators (EMA, RSI, MACD, etc.).                                      |
//! | [`rayon`](https://crates.io/crates/rayon) | Parallel processing for optimization.                                                     |
//! | [`serde`](https://crates.io/crates/serde) | Serialize/deserialize backtest results.                                                    |
//! | [`plotters`](https://crates.io/crates/plotters) | Visualize equity curves and indicators.                                                   |
//!
//! ## Error Handling
//! BTS uses custom error types to handle:
//! - Insufficient balance.
//! - Invalid order types.
//! - Missing data (e.g., candles, positions).
//!
//! Example:
//! ```rust
//! use bts::prelude::*;
//! use chrono::DateTime;
//!
//! fn main() {
//!     let candle = CandleBuilder::builder()
//!         .open(100.0)
//!         .high(110.0)
//!         .low(95.0)
//!         .close(105.0)
//!         .volume(1.0)
//!         .bid(0.5)
//!         .open_time(DateTime::default())
//!         .close_time(DateTime::default())
//!         .build()
//!         .unwrap();
//!
//!     // Initialize backtest with \$10,000
//!     let mut backtest = Backtest::new(vec![candle], 10_000.0, None).unwrap();
//!
//!     // Execute a market buy order
//!     backtest
//!         .run(|bt, _candle| {
//!             let order: Order = (OrderType::Market(102.0), 1.0, OrderSide::Buy).into();
//!             match bt.place_order(order) {
//!                Ok(_) => println!("Order in the pool!"),
//!                Err(_) => eprintln!("Error to place an order")
//!             }
//!             Ok(())
//!         })
//!         .unwrap();
//! }
//! ```
//!
//! ## Contributing
//! Contributions are welcome! See [`CONTRIBUTING.md`](https://github.com/raonagos/bts/blob/main/CONTRIBUTING.md).
//!
//! ## License
//! MIT
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

/// Core trading engine components: orders, positions, wallet, and backtest logic.
pub mod engine;

/// Error types for the library.
pub mod errors;

/// Utility functions and helpers.
mod utils;

/// Performance metrics: drawdown, Sharpe ratio, win rate, etc.
#[cfg(feature = "metrics")]
pub mod metrics;

/// Re-exports of commonly used types and traits for convenience.
pub mod prelude {
    pub use super::*;
    pub use crate::engine::*;
    pub use crate::errors::*;

    #[cfg(feature = "metrics")]
    pub use crate::metrics::*;
}

use std::ops::{Add, Div, Mul, Sub};

/// Trait for performing percentage-based calculations.
///
/// This trait provides methods to add, subtract, and calculate percentages
/// for numeric types, enabling common financial calculations.
pub trait PercentCalculus<Rhs = Self> {
    /// Adds a percentage to the value.
    ///
    /// ### Arguments
    /// * `rhs` - The percentage to add (e.g., 10.0 for 10%).
    ///
    /// ### Returns
    /// The value increased by the given percentage.
    fn addpercent(self, rhs: Rhs) -> Self;

    /// Subtracts a percentage from the value.
    ///
    /// ### Arguments
    /// * `rhs` - The percentage to subtract (e.g., 10.0 for 10%).
    ///
    /// ### Returns
    /// The value decreased by the given percentage.
    fn subpercent(self, rhs: Rhs) -> Self;

    /// Calculates the absolute value of a percentage.
    ///
    /// ### Arguments
    /// * `percent` - The percentage to calculate (e.g., 10.0 for 10%).
    ///
    /// ### Returns
    /// The absolute value of the given percentage.
    fn how_many(self, percent: Self) -> Self;

    /// Calculates the percentage change between two values.
    ///
    /// ### Arguments
    /// * `new` - The new value to compare with.
    ///
    /// ### Returns
    /// The percentage change from the original value to the new value.
    fn change(self, new: Self) -> Self;
}

impl PercentCalculus for f64 {
    fn addpercent(self, percent: Self) -> Self {
        self.add(self.mul(percent.div(100.0)))
    }

    fn subpercent(self, percent: Self) -> Self {
        self.sub(self.mul(percent.div(100.0)))
    }

    fn how_many(self, percent: Self) -> Self {
        percent.mul(self.div(100.0))
    }

    fn change(self, new: Self) -> Self {
        new.sub(self).div(self).mul(100.0)
    }
}

#[cfg(test)]
mod percent {
    use super::*;

    #[test]
    fn add() {
        assert_eq!(110.0, 100.0.addpercent(10.0))
    }

    #[test]
    fn sub() {
        assert_eq!(90.0, 100.0.subpercent(10.0))
    }

    #[test]
    fn how_many() {
        assert_eq!(10.0, 100.0.how_many(10.0))
    }

    #[test]
    fn change() {
        assert_eq!(10.0, 100.0.change(110.0))
    }
}
