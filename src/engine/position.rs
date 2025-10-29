use rand::Rng;

#[derive(Debug, Clone, PartialEq)]
pub enum PositionSide {
    Long,
    Short,
}

#[derive(Debug, Clone)]
pub struct Position {
    id: u32,
    side: PositionSide,
    entry_price: f64,
    quantity: f64,
}

impl Position {
    pub fn random_id() -> u32 {
        let mut rng = rand::rng();
        rng.random_range(1..1000)
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn side(&self) -> PositionSide {
        self.side.clone()
    }

    pub fn quantity(&self) -> f64 {
        self.quantity
    }

    pub fn entry_price(&self) -> f64 {
        self.entry_price
    }

    pub fn estimate_profit(&self, exit_price: f64) -> f64 {
        match self.side {
            PositionSide::Long => (exit_price - self.entry_price) * self.quantity,
            PositionSide::Short => (self.entry_price - exit_price) * self.quantity,
        }
    }
}

impl From<(PositionSide, f64, f64)> for Position {
    fn from((side, entry_price, quantity): (PositionSide, f64, f64)) -> Self {
        Self {
            id: Self::random_id(),
            side,
            entry_price,
            quantity,
        }
    }
}

impl From<(u32, PositionSide, f64, f64)> for Position {
    fn from((id, side, entry_price, quantity): (u32, PositionSide, f64, f64)) -> Self {
        Self {
            id,
            side,
            entry_price,
            quantity,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PositionEventType {
    Open(PositionSide),
    Close,
}

#[derive(Debug, Clone)]
pub struct PositionEvent {
    candle_index: usize,
    price: f64,
    event_type: PositionEventType,
}

impl From<(usize, f64, PositionEventType)> for PositionEvent {
    fn from((index, price, event): (usize, f64, PositionEventType)) -> Self {
        Self {
            candle_index: index,
            price,
            event_type: event,
        }
    }
}
