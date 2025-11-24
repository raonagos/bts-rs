#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{errors::*, utils::random_id};

/// Represents the side of an order (buy or sell).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub enum OrderSide {
    /// A buy order, where the trader wants to purchase an asset.
    Buy,
    /// A sell order, where the trader wants to sell an asset.
    Sell,
}

/// Represents the type of an order (market, limit, take-profit/stop-loss, trailing stop).
/// Enum representing the type of an order.
///
/// This enum is divided into two categories:
/// 1. **Order types for opening positions** (Market, Limit)
/// 2. **Exit rules for closing positions** (TakeProfit, StopLoss, TrailingStop)
///
/// This separation ensures clarity between order types used to open positions
/// and rules used to automatically close them.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum OrderType {
    /// Market order to open a position immediately at the current price.
    ///
    /// ### Arguments
    /// * `0` - The price at which the market order should be executed.
    Market(f64),

    /// Limit order to open a position only at a specified price or better.
    ///
    /// ### Arguments
    /// * `0` - The limit price for the order.
    Limit(f64),

    /// Combined take-profit and stop-loss exit rule for a position.
    ///
    /// When either the take-profit or stop-loss price is reached, the position will be closed.
    ///
    /// ### Arguments
    /// * `0` - The take-profit price (0.0 to disable)
    /// * `1` - The stop-loss price (0.0 to disable)
    TakeProfitAndStopLoss(f64, f64),

    /// Trailing stop exit rule for a position.
    ///
    /// The stop price trails the market price by a specified percentage.
    /// For long positions, the stop moves up as the price increases.
    /// For short positions, the stop moves down as the price decreases.
    ///
    /// ### Arguments
    /// * `0` - The initial stop price
    /// * `1` - The trailing percentage (e.g., 10.0 for 10%)
    TrailingStop(f64, f64),
}

impl OrderType {
    /// Returns the price associated with the order type (for Market and Limit orders).
    pub fn inner(&self) -> Result<f64> {
        match self {
            Self::Market(price) | Self::Limit(price) => Ok(*price),
            _ => Err(Error::MismatchedOrderType),
        }
    }
}

/// Represents an order with entry and exit rules.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Order {
    id: u32,
    entry_type: OrderType,
    /// Represents the volume of the order.
    pub quantity: f64,
    /// Represents the buy/sell side of the order.
    pub side: OrderSide,
    exit_type: Option<OrderType>,
}

impl PartialEq for Order {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

type O1 = (OrderType, f64, OrderSide);
impl From<O1> for Order {
    fn from((entry_type, quantity, side): O1) -> Self {
        Self {
            id: random_id(),
            entry_type,
            quantity,
            side,
            exit_type: None,
        }
    }
}

type O2 = (OrderType, OrderType, f64, OrderSide);
impl From<O2> for Order {
    fn from((entry_type, exit_type, quantity, side): O2) -> Self {
        Self {
            id: random_id(),
            entry_type,
            quantity,
            side,
            exit_type: Some(exit_type),
        }
    }
}

impl Order {
    /// Returns the entry price of the order.
    pub fn entry_price(&self) -> Result<f64> {
        self.entry_type.inner()
    }

    /// Returns the total cost of the order (price * quantity).
    pub(crate) fn cost(&self) -> Result<f64> {
        let inner = self.entry_type.inner()?;
        Ok(inner * self.quantity)
    }

    /// Returns the entry type of the order.
    pub fn entry_type(&self) -> &OrderType {
        &self.entry_type
    }

    /// Returns the exit rule of the order, if any.
    pub fn exit_rule(&self) -> &Option<OrderType> {
        &self.exit_type
    }

    /// Returns true if it is a market order, and false if it is a limit order.
    pub fn is_market_type(&self) -> bool {
        matches!(self.entry_type, OrderType::Market(_))
    }

    /// Updates the trailing stop price for the order.
    pub fn set_trailingstop(&mut self, new_price: f64) {
        if let Some(OrderType::TrailingStop(current_price, _)) = &mut self.exit_type {
            match self.side {
                OrderSide::Buy => {
                    if new_price > *current_price {
                        *current_price = new_price;
                    }
                }
                OrderSide::Sell => {
                    if new_price < *current_price {
                        *current_price = new_price;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[test]
fn create_simple_order() {
    let entry_type = OrderType::Market(100.0);
    let quantity = 2.0;
    let side = OrderSide::Buy;
    let order: Order = (entry_type, quantity, side).into();

    assert_eq!(order.entry_price().unwrap(), 100.0);
    assert_eq!(order.quantity, 2.0);
    assert_eq!(order.cost().unwrap(), 200.0);
    assert!(matches!(order.side, OrderSide::Buy));
    assert!(order.exit_rule().is_none());
}

#[cfg(test)]
#[test]
fn create_order_with_exit_rule() {
    let entry_type = OrderType::Limit(100.0);
    let exit_type = OrderType::TakeProfitAndStopLoss(120.0, 90.0);
    let quantity = 1.5;
    let side = OrderSide::Sell;
    let order: Order = (entry_type, exit_type, quantity, side).into();

    assert_eq!(order.entry_price().unwrap(), 100.0);
    assert_eq!(order.quantity, 1.5);
    assert_eq!(order.cost().unwrap(), 150.0);
    assert!(matches!(order.side, OrderSide::Sell));
    assert!(matches!(
        order.exit_rule(),
        Some(OrderType::TakeProfitAndStopLoss(120.0, 90.0))
    ));
}

#[cfg(test)]
#[test]
fn order_equality() {
    let order1: Order = (OrderType::Market(100.0), 1.0, OrderSide::Buy).into();
    let order2: Order = (OrderType::Market(100.0), 1.0, OrderSide::Buy).into();
    assert_ne!(order1, order2);
    assert_eq!(order1, order1);
}

#[cfg(test)]
#[test]
fn entry_price() {
    let market_order: Order = (OrderType::Market(100.0), 1.0, OrderSide::Buy).into();
    assert_eq!(market_order.entry_price().unwrap(), 100.0);

    let limit_order: Order = (OrderType::Limit(150.0), 1.0, OrderSide::Sell).into();
    assert_eq!(limit_order.entry_price().unwrap(), 150.0);
}

#[cfg(test)]
#[test]
fn order_cost() {
    let order: Order = (OrderType::Market(100.0), 2.5, OrderSide::Buy).into();
    assert_eq!(order.cost().unwrap(), 250.0);

    let order: Order = (OrderType::Limit(200.0), 0.5, OrderSide::Sell).into();
    assert_eq!(order.cost().unwrap(), 100.0);
}

#[cfg(test)]
#[test]
fn set_trailingstop_buy() {
    let mut order: Order = (
        OrderType::Market(100.0),
        OrderType::TrailingStop(95.0, 5.0),
        1.0,
        OrderSide::Buy,
    )
        .into();

    order.set_trailingstop(90.0);
    if let Some(OrderType::TrailingStop(price, _)) = order.exit_rule() {
        assert_eq!(*price, 95.0);
    } else {
        panic!("Expected TrailingStop order type");
    }

    order.set_trailingstop(105.0);
    if let Some(OrderType::TrailingStop(price, _)) = order.exit_rule() {
        assert_eq!(*price, 105.0);
    } else {
        panic!("Expected TrailingStop order type");
    }
}

#[cfg(test)]
#[test]
fn set_trailingstop_sell() {
    let mut order: Order = (
        OrderType::Market(100.0),
        OrderType::TrailingStop(105.0, 5.0),
        1.0,
        OrderSide::Sell,
    )
        .into();

    order.set_trailingstop(110.0);
    if let Some(OrderType::TrailingStop(price, _)) = order.exit_rule() {
        assert_eq!(*price, 105.0);
    } else {
        panic!("Expected TrailingStop order type");
    }

    order.set_trailingstop(95.0);
    if let Some(OrderType::TrailingStop(price, _)) = order.exit_rule() {
        assert_eq!(*price, 95.0);
    } else {
        panic!("Expected TrailingStop order type");
    }
}

#[cfg(test)]
#[test]
fn set_trailingstop_no_exit_rule() {
    let mut order: Order = (OrderType::Market(100.0), 1.0, OrderSide::Buy).into();
    order.set_trailingstop(150.0);
    assert!(order.exit_rule().is_none());
}

#[cfg(test)]
#[test]
fn order_type_inner() {
    let market_order = OrderType::Market(100.0);
    assert_eq!(market_order.inner().unwrap(), 100.0);

    let limit_order = OrderType::Limit(150.0);
    assert_eq!(limit_order.inner().unwrap(), 150.0);
}

#[cfg(test)]
#[test]
#[should_panic]
fn order_type_inner_panics() {
    let take_profit_order = OrderType::TakeProfitAndStopLoss(120.0, 90.0);
    take_profit_order.inner().unwrap();
}
