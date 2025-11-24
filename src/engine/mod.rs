//! Core trading engine components.
//!
//! This module provides the fundamental types for backtesting:
//! - `Order`: Market, limit, and conditional orders.
//! - `Position`: Open trades with exit rules.
//! - `Wallet`: Tracks balance, fees, and P&L.
//! - `Candle`: OHLCV data for backtesting.

mod candle;
mod order;
mod position;
mod wallet;

use std::collections::{VecDeque, vec_deque::Iter};

use crate::{
    PercentCalculus,
    errors::{Error, Result},
};

#[cfg(feature = "metrics")]
use crate::metrics::*;

pub use candle::*;
pub use order::*;
pub use position::*;
use wallet::*;

#[cfg(test)]
mod bts;

#[cfg(test)]
impl Iterator for Backtest {
    type Item = Candle;

    fn next(&mut self) -> Option<Self::Item> {
        let candle = self.data.get(self.index).cloned();
        self.index += 1;
        candle
    }
}

/// Backtesting engine for trading strategies.
#[derive(Debug)]
pub struct Backtest {
    index: usize,
    wallet: Wallet,
    data: Vec<Candle>,
    #[cfg(feature = "metrics")]
    events: Vec<Event>,
    orders: VecDeque<Order>,
    positions: VecDeque<Position>,
    market_fees: Option<(f64, f64)>,
}

impl std::ops::Deref for Backtest {
    type Target = Wallet;

    fn deref(&self) -> &Self::Target {
        &self.wallet
    }
}

impl Backtest {
    /// Creates a new backtest instance.
    ///
    /// ### Arguments
    /// * `data` - Vector of candle data.
    /// * `initial_balance` - Initial wallet balance.
    /// * `market_fee` - Market *(market and limit)* fee percentage (e.g., 0.1 for 0.1%).
    ///   Fees are **only applied when positions are opened**, not when orders are placed.
    ///
    /// ### Market Fees Behavior
    /// - **Order Placement**: No fees are charged when placing an order.
    ///   Fees are only deducted when the order is executed and a position is opened.
    /// - **Position Opening**: Fees are calculated as `price × quantity × market_fee`
    ///   and deducted from the wallet when the position is opened.
    /// - **Order Cancellation**: No fees are charged if an order is cancelled before execution.
    ///
    /// ### Returns
    /// The new backtest instance or an error.
    pub fn new(data: Vec<Candle>, initial_balance: f64, market_fees: Option<(f64, f64)>) -> Result<Self> {
        if data.is_empty() {
            return Err(Error::CandleDataEmpty);
        }

        if let Some((market_fee, limit_fee)) = market_fees {
            if market_fee <= 0.0 || limit_fee <= 0.0 {
                return Err(Error::NegZeroFees);
            }
        }

        Ok(Self {
            data,
            index: 0,
            market_fees,
            #[cfg(feature = "metrics")]
            events: Vec::new(),
            orders: VecDeque::new(),
            positions: VecDeque::new(),
            wallet: Wallet::new(initial_balance)?,
        })
    }

    /// Returns an iterator over the pending orders.
    pub fn orders(&self) -> Iter<'_, Order> {
        self.orders.iter()
    }

    /// Returns an iterator over the open positions.
    pub fn positions(&self) -> Iter<'_, Position> {
        self.positions.iter()
    }

    /// Returns an iterator over the recorded events.
    #[cfg(feature = "metrics")]
    pub fn events(&self) -> std::slice::Iter<'_, Event> {
        self.events.iter()
    }

    /// Places a new order.
    ///
    /// ### Arguments
    /// * `order` - The order to place.
    ///
    /// ### Returns
    /// Ok if successful, or an error.
    pub fn place_order(&mut self, order: Order) -> Result<()> {
        self.wallet.lock(order.cost()?)?;
        self.orders.push_back(order.clone());
        #[cfg(feature = "metrics")]
        {
            let wallet_event = Event::WalletUpdate {
                locked: self.wallet.locked(),
                fees: self.wallet.fees_paid(),
                balance: self.wallet.balance(),
                pnl: self.wallet.unrealized_pnl(),
                free: self.wallet.free_balance()?,
            };
            self.events.push(wallet_event);
            self.events.push(Event::AddOrder(order));
        }
        Ok(())
    }

    /// Deletes a pending order.
    ///
    /// ### Arguments
    /// * `order` - Reference to the order to delete.
    ///
    /// ### Returns
    /// Ok if successful, or an error.
    pub fn delete_order(&mut self, order: &Order, force_remove: bool) -> Result<()> {
        if force_remove {
            let order_idx = self
                .orders
                .iter()
                .position(|o| o == order)
                .ok_or(Error::OrderNotFound)?;
            self.orders.remove(order_idx).ok_or(Error::RemoveOrder)?;
        }
        self.wallet.unlock(order.cost()?)?;
        #[cfg(feature = "metrics")]
        {
            let wallet_event = Event::WalletUpdate {
                locked: self.wallet.locked(),
                fees: self.wallet.fees_paid(),
                balance: self.wallet.balance(),
                free: self.wallet.free_balance()?,
                pnl: self.wallet.unrealized_pnl(),
            };
            self.events.push(wallet_event);
            self.events.push(Event::DelOrder(order.clone()));
        }
        Ok(())
    }

    /// Opens a new position.
    fn open_position(&mut self, position: Position) -> Result<()> {
        self.wallet.sub(position.cost()?)?;
        if let Some((market_fee, limit_fee)) = self.market_fees {
            if position.is_market_type() {
                self.wallet.sub_fees(position.cost()? * market_fee)?;
            } else {
                self.wallet.sub_fees(position.cost()? * limit_fee)?;
            };
        }
        self.positions.push_back(position.clone());
        #[cfg(feature = "metrics")]
        {
            let wallet_event = Event::WalletUpdate {
                locked: self.wallet.locked(),
                fees: self.wallet.fees_paid(),
                balance: self.wallet.balance(),
                free: self.wallet.free_balance()?,
                pnl: self.wallet.unrealized_pnl(),
            };
            self.events.push(wallet_event);
            self.events.push(Event::AddPosition(position));
        }
        Ok(())
    }

    /// Closes an existing position.
    ///
    /// ### Arguments
    /// * `position` - Reference to the position to close.
    /// * `exit_price` - The price at which to close the position.
    /// * `force_remove` - If true, removes the position without checking conditions.
    ///
    /// ### Returns
    /// The profit/loss from closing the position, or an error.
    pub fn close_position(&mut self, position: &Position, exit_price: f64, force_remove: bool) -> Result<f64> {
        if exit_price <= 0.0 || !exit_price.is_finite() {
            return Err(Error::ExitPrice(exit_price));
        }
        if force_remove {
            let pos_idx = self
                .positions
                .iter()
                .position(|p| p == position)
                .ok_or(Error::PositionNotFound)?;
            self.positions.remove(pos_idx).ok_or(Error::RemovePosition)?;
        }
        // Calculate profit/loss and update wallet
        let pnl = position.estimate_pnl(exit_price)?;
        let total_amount = pnl + position.cost()?;
        self.wallet.add(total_amount)?;
        self.wallet.sub_pnl(total_amount);
        if let Some((market_fee, limit_fee)) = self.market_fees {
            if position.is_market_type() {
                self.wallet.sub_fees(position.cost()? * market_fee)?;
            } else {
                self.wallet.sub_fees(position.cost()? * limit_fee)?;
            };
        }
        #[cfg(feature = "metrics")]
        {
            let wallet_event = Event::WalletUpdate {
                locked: self.wallet.locked(),
                fees: self.wallet.fees_paid(),
                balance: self.wallet.balance(),
                free: self.wallet.free_balance()?,
                pnl: self.wallet.unrealized_pnl(),
            };
            self.events.push(wallet_event);
            let mut position = position.clone();
            position.set_exit_price(exit_price)?;
            self.events.push(Event::DelPosition(position));
        }
        Ok(pnl)
    }

    /// Closes all open positions at the given exit price.
    ///
    /// ### Arguments
    /// * `exit_price` - The price at which to close all positions.
    ///
    /// ### Returns
    /// Ok if successful, or an error.
    pub fn close_all_positions(&mut self, exit_price: f64) -> Result<()> {
        while let Some(position) = self.positions.pop_front() {
            self.close_position(&position, exit_price, false)?;
        }
        Ok(())
    }

    /// Executes pending orders based on current candle data.
    fn execute_orders(&mut self, candle: &Candle) -> Result<()> {
        let mut orders = VecDeque::with_capacity(self.orders.len());
        while let Some(order) = self.orders.pop_front() {
            let price = order.entry_price()?;
            if price >= candle.low() && price <= candle.high() {
                self.open_position(Position::from(order))?;
            } else {
                //? if order is market type and does not between `high` and `low`, delete
                if order.is_market_type() {
                    self.delete_order(&order, false)?;
                } else {
                    orders.push_back(order);
                }
            }
        }
        self.orders.append(&mut orders);
        Ok(())
    }

    /// Executes position management (take-profit, stop-loss, trailing stop).
    fn execute_positions(&mut self, candle: &Candle) -> Result<()> {
        let mut positions = VecDeque::with_capacity(self.positions.len());

        while let Some(mut position) = self.positions.pop_front() {
            let should_close = match position.exit_rule() {
                Some(OrderType::TakeProfitAndStopLoss(take_profit, stop_loss)) => {
                    if *take_profit < 0.0 || *stop_loss < 0.0 {
                        return Err(Error::NegTakeProfitAndStopLoss);
                    }

                    match position.side {
                        PositionSide::Long => {
                            if *take_profit > 0.0 && take_profit <= &candle.high() {
                                Some(*take_profit)
                            } else if *stop_loss > 0.0 && stop_loss >= &candle.low() {
                                Some(*stop_loss)
                            } else {
                                None
                            }
                        }
                        PositionSide::Short => {
                            if *take_profit > 0.0 && take_profit >= &candle.low() {
                                Some(*take_profit)
                            } else if *stop_loss > 0.0 && stop_loss <= &candle.high() {
                                Some(*stop_loss)
                            } else {
                                None
                            }
                        }
                    }
                }
                Some(OrderType::TrailingStop(price, percent)) => {
                    if *price <= 0.0 || *percent <= 0.0 {
                        return Err(Error::NegZeroTrailingStop);
                    }

                    match position.side {
                        PositionSide::Long => {
                            let execute_price = price.subpercent(*percent);
                            if execute_price >= candle.low() {
                                Some(execute_price)
                            } else {
                                if &candle.high() > price {
                                    position.set_trailingstop(candle.high());
                                }
                                None
                            }
                        }
                        PositionSide::Short => {
                            let execute_price = price.addpercent(*percent);
                            if execute_price <= candle.high() {
                                Some(execute_price)
                            } else {
                                if &candle.low() < price {
                                    position.set_trailingstop(candle.low());
                                }
                                None
                            }
                        }
                    }
                }
                None => None,
                _ => {
                    return Err(Error::MismatchedOrderType);
                }
            };

            match should_close {
                Some(exit_price) => {
                    self.close_position(&position, exit_price, false)?;
                }
                None => positions.push_back(position),
            }
        }

        let mut total_unrealized_pnl = 0.0;
        for position in &positions {
            // calculate unrealized P&L for this position
            let current_price = candle.close();
            let pnl = position.estimate_pnl(current_price)?;
            total_unrealized_pnl += pnl;
        }

        self.positions.append(&mut positions);
        self.wallet.set_unrealized_pnl(total_unrealized_pnl);
        Ok(())
    }

    /// Runs the backtest, executing the provided function for each candle.
    ///
    /// ### Arguments
    /// * `func` - A closure that takes the backtest and current candle.
    ///
    /// ### Returns
    /// Ok if successful, or an error.
    pub fn run<F>(&mut self, mut func: F) -> Result<()>
    where
        F: FnMut(&mut Self, &Candle) -> Result<()>,
    {
        while self.index < self.data.len() {
            let candle = self.data.get(self.index).ok_or(Error::CandleNotFound)?.clone();
            func(self, &candle)?;
            self.execute_orders(&candle)?;
            self.execute_positions(&candle)?;
            self.index += 1;
        }

        Ok(())
    }

    /// Resets the backtest to its initial state.
    pub fn reset(&mut self) {
        self.index = 0;
        self.wallet.reset();
        #[cfg(feature = "metrics")]
        {
            self.events = Vec::new();
        }
        self.orders = VecDeque::new();
        self.positions = VecDeque::new();
    }
}
