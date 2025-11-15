use super::order::{Order, OrderSide};

/// Represents the side of a position (long or short).
#[derive(Debug, Clone, PartialEq)]
pub enum PositionSide {
    Long,
    Short,
}

/// Represents a trading position with an associated order.
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    order: Order,
    pub side: PositionSide,
}

impl From<Order> for Position {
    fn from(value: Order) -> Self {
        Self {
            side: match value.side {
                OrderSide::Buy => PositionSide::Long,
                OrderSide::Sell => PositionSide::Short,
            },
            order: value,
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
    pub fn estimate_profit(&self, exit_price: f64) -> f64 {
        match self.side {
            PositionSide::Long => (exit_price - self.entry_price()) * self.quantity,
            PositionSide::Short => (self.entry_price() - exit_price) * self.quantity,
        }
    }
}
