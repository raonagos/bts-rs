//! Core trading engine components.
//!
//! This module provides the fundamental types for backtesting:
//! - `Order`: Market, limit, and conditional orders.
//! - `Position`: Open trades with exit rules.
//! - `Wallet`: Tracks balance, fees, and P&L.
//! - `Candle`: OHLCV data for backtesting.
//! - `Backtest`: The engine to run the backtest.

mod bts;
mod candle;
mod order;
mod position;
mod wallet;

pub use bts::*;
pub use candle::*;
pub use order::*;
pub use position::*;
pub(crate) use wallet::*;
