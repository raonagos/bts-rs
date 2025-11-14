use crate::engine::{Position, PositionSide};

#[derive(Debug, Clone)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl Into<PositionSide> for OrderSide {
    fn into(self) -> PositionSide {
        match self {
            OrderSide::Buy => PositionSide::Long,
            OrderSide::Sell => PositionSide::Short,
        }
    }
}

#[derive(Debug, Clone)]
pub enum OrderType {
    Market(f64),
    Limit(f64),
}

impl OrderType {
    pub fn inner(&self) -> f64 {
        match self {
            Self::Market(price) | Self::Limit(price) => price.to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Order {
    _type: OrderType,
    pub quantity: f64,
    pub side: OrderSide,
}

impl Into<Position> for Order {
    fn into(self) -> Position {
        //! maybe this `into()` looping
        Position::from((self.side.into(), self._type.inner(), self.quantity))
    }
}

impl Order {
    pub fn reverse_position(position: &Position) -> Self {
        Self {
            _type: OrderType::Market(position.entry_price),
            quantity: -position.quantity,
            side: match position.side {
                PositionSide::Long => OrderSide::Sell,
                PositionSide::Short => OrderSide::Buy,
            },
        }
    }

    pub fn entry_price(&self) -> f64 {
        self._type.inner()
    }

    pub(crate) fn cost(&self) -> f64 {
        self._type.inner() * self.quantity
    }

    pub fn type_(&self) -> &OrderType {
        &self._type
    }
}
