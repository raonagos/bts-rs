use super::order::{Order, OrderSide};
use crate::{errors::*, utils::random_id};

/// Represents the side of a position (long or short).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy)]
pub enum PositionSide {
    /// A long position, where the trader buys an asset with the expectation that its price will increase.
    Long,
    /// A short position, where the trader sells an asset (borrowed or owned) with the expectation that its price will decrease.
    Short,
}

/// Represents a trading position with an associated order.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy)]
pub struct Position {
    id: u32,
    order: Order,
    side: PositionSide,
    #[cfg(feature = "metrics")]
    exit_price: Option<f64>,
}

impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl From<Order> for Position {
    fn from(value: Order) -> Self {
        Self {
            id: random_id(),
            #[cfg(feature = "metrics")]
            exit_price: None,
            order: value,
            side: match value.side() {
                OrderSide::Buy => PositionSide::Long,
                OrderSide::Sell => PositionSide::Short,
            },
        }
    }
}

impl std::ops::Deref for Position {
    type Target = Order;
    fn deref(&self) -> &Self::Target {
        &self.order
    }
}

impl std::ops::DerefMut for Position {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.order
    }
}

impl Position {
    /// Returns the position side.
    pub fn side(&self) -> &PositionSide {
        &self.side
    }

    /// Returns the current exit price.
    #[cfg(feature = "metrics")]
    pub fn exit_price(&self) -> Option<&f64> {
        self.exit_price.as_ref()
    }

    #[cfg(feature = "metrics")]
    /// Updates the `exit_price`.
    pub(crate) fn set_exit_price(&mut self, exit_price: f64) -> Result<()> {
        if exit_price < 0.0 {
            return Err(Error::ExitPrice(exit_price));
        }
        self.exit_price = Some(exit_price);
        Ok(())
    }

    #[cfg(feature = "metrics")]
    /// Returns the estimated profit and loss if it is closed at the `exit_price`.
    pub fn pnl(&self) -> Result<f64> {
        let exit_price = self.exit_price.ok_or(Error::ExitPrice(0.0))?;
        self.estimate_pnl(exit_price)
    }

    /// Returns the estimated profit and loss if it is closed at the `exit_price`.
    pub fn estimate_pnl(&self, exit_price: f64) -> Result<f64> {
        let pnl = match self.side {
            PositionSide::Long => (exit_price - self.entry_price()?) * self.quantity(),
            PositionSide::Short => (self.entry_price()? - exit_price) * self.quantity(),
        };
        Ok(pnl)
    }
}

#[cfg(test)]
use super::order::OrderType;

#[cfg(test)]
#[test]
fn create_position_from_buy_order() {
    let order: Order = (OrderType::Market(100.0), 2.0, OrderSide::Buy).into();
    let position = Position::from(order);

    assert_eq!(position.entry_price().unwrap(), 100.0);
    assert_eq!(position.quantity(), 2.0);
    assert!(matches!(position.side, PositionSide::Long));
    assert_eq!(position.cost().unwrap(), 200.0);
}

#[cfg(test)]
#[test]
fn create_position_from_sell_order() {
    let order: Order = (OrderType::Limit(150.0), 1.5, OrderSide::Sell).into();
    let position = Position::from(order);

    assert_eq!(position.entry_price().unwrap(), 150.0);
    assert_eq!(position.quantity(), 1.5);
    assert!(matches!(position.side, PositionSide::Short));
    assert_eq!(position.cost().unwrap(), 225.0);
}

#[cfg(test)]
#[test]
fn position_equality() {
    let order1: Order = (OrderType::Market(100.0), 1.0, OrderSide::Buy).into();
    let position1 = Position::from(order1);
    let order2: Order = (OrderType::Market(100.0), 1.0, OrderSide::Buy).into();
    let position2 = Position::from(order2);

    assert_ne!(position1, position2);
    assert_eq!(position1, position1);
}

#[cfg(test)]
#[test]
fn position_ids_are_unique() {
    let order1: Order = (OrderType::Market(100.0), 1.0, OrderSide::Buy).into();
    let position1 = Position::from(order1);
    let order2: Order = (OrderType::Market(100.0), 1.0, OrderSide::Buy).into();
    let position2 = Position::from(order2);

    assert_ne!(position1.id, position2.id);
}

#[cfg(test)]
#[test]
fn position_deref() {
    let order: Order = (OrderType::Market(100.0), 2.0, OrderSide::Buy).into();
    let position = Position::from(order);

    assert_eq!(position.entry_price().unwrap(), 100.0);
    assert_eq!(position.quantity(), 2.0);
    assert!(matches!(position.side, PositionSide::Long));
}

#[cfg(test)]
#[test]
fn position_deref_mut() {
    let order: Order = (OrderType::Market(100.0), 2.0, OrderSide::Buy).into();
    let mut position = Position::from(order);

    position.set_quantity(3.0);
    assert_eq!(position.quantity(), 3.0);
}

#[cfg(test)]
#[test]
fn estimate_pnl_long_position() {
    let order: Order = (OrderType::Market(100.0), 2.0, OrderSide::Buy).into();
    let position = Position::from(order);

    assert_eq!(position.estimate_pnl(120.0).unwrap(), 40.0);
    assert_eq!(position.estimate_pnl(80.0).unwrap(), -40.0);
    assert_eq!(position.estimate_pnl(100.0).unwrap(), 0.0);
}

#[cfg(test)]
#[test]
fn estimate_pnl_short_position() {
    let order: Order = (OrderType::Market(100.0), 2.0, OrderSide::Sell).into();
    let position = Position::from(order);

    assert_eq!(position.estimate_pnl(80.0).unwrap(), 40.0);
    assert_eq!(position.estimate_pnl(120.0).unwrap(), -40.0);
    assert_eq!(position.estimate_pnl(100.0).unwrap(), 0.0);
}

#[cfg(test)]
#[test]
fn position_with_exit_rule() {
    let order: Order = (
        OrderType::Limit(100.0),
        OrderType::TakeProfitAndStopLoss(120.0, 90.0),
        1.5,
        OrderSide::Sell,
    )
        .into();
    let position = Position::from(order);

    assert_eq!(position.entry_price().unwrap(), 100.0);
    assert_eq!(position.quantity(), 1.5);
    assert!(matches!(position.side, PositionSide::Short));
    assert!(matches!(
        position.exit_rule(),
        Some(OrderType::TakeProfitAndStopLoss(120.0, 90.0))
    ));
}

#[cfg(test)]
#[test]
fn position_set_trailingstop() {
    let order: Order = (
        OrderType::Market(100.0),
        OrderType::TrailingStop(95.0, 5.0),
        1.0,
        OrderSide::Buy,
    )
        .into();
    let mut position = Position::from(order);

    position.set_trailingstop(105.0);
    if let Some(OrderType::TrailingStop(price, _)) = position.exit_rule() {
        assert_eq!(*price, 105.0);
    } else {
        panic!("Expected TrailingStop order type");
    }
}
