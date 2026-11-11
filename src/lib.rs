#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

/// Core trading engine components: orders, positions, wallet, and backtest logic.
pub mod engine;

/// Error types for the library.
pub mod errors;

/// Utility functions and helpers.
mod utils;

/// Performance metrics: drawdown, Sharpe ratio, win rate, etc.
#[cfg(feature = "metrics")]
pub mod metrics;

/// Strategy parameter optimization.
#[cfg(feature = "optimizer")]
pub mod optimizer;

/// Module for visualizing backtest results and candle charts.
#[cfg(feature = "draws")]
pub mod draws;

/// Re-exports of commonly used types and traits for convenience.
pub mod prelude {
    pub use super::PercentCalculus;
    pub use crate::engine::*;

    #[cfg(feature = "metrics")]
    pub use crate::metrics::*;

    #[cfg(feature = "optimizer")]
    pub use crate::optimizer::*;

    #[cfg(feature = "draws")]
    pub use crate::draws::*;
}

use std::ops::{Add, Div, Mul, Sub};

/// Trait for performing percentage-based calculations (human readable).
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
    /// * `rhs` - The percentage to calculate (e.g., 10.0 for 10%).
    ///
    /// ### Returns
    /// The absolute value of the given percentage.
    fn how_many(self, rhs: Rhs) -> Self;

    /// Calculates the percentage change between two values.
    ///
    /// ### Arguments
    /// * `value` - The value to compare with.
    ///
    /// ### Returns
    /// The percentage change from the original value to the input value.
    fn change(self, value: Rhs) -> Self;
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

    fn change(self, value: Self) -> Self {
        value.sub(self).div(self).mul(100.0)
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
