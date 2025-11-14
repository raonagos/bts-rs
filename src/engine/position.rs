#[derive(Debug, Clone, PartialEq)]
pub enum PositionSide {
    Long,
    Short,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub entry_price: f64,
    pub quantity: f64,
    pub side: PositionSide,
}

impl Position {
    pub fn cost(&self) -> f64 {
        self.entry_price * self.quantity
    }

    pub fn estimate_profit(&self, exit_price: f64) -> f64 {
        match self.side {
            PositionSide::Long => (exit_price - self.entry_price) * self.quantity,
            PositionSide::Short => (self.entry_price - exit_price) * self.quantity,
        }
    }

    pub fn profit_change(&self, exit_price: f64) -> f64 {
        let mut v1 = self.entry_price * self.quantity;
        let mut v2 = exit_price * self.quantity;
        if self.side == PositionSide::Short {
            let temp = v1;
            v1 = v2;
            v2 = temp;
        }
        (v2 - v1) / v1 * 100.0
    }
}

type P1 = (PositionSide, f64, f64);
impl From<P1> for Position {
    fn from((side, entry_price, quantity): P1) -> Self {
        Self {
            side,
            quantity,
            entry_price,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PositionEvent {
    open: (usize, PositionSide, f64),
    close: Option<(usize, f64)>,
}

impl PositionEvent {
    pub fn len(&self) -> Option<usize> {
        self.close.map(|(pos_idx, _)| pos_idx - self.open.0)
    }

    pub fn close(&mut self, pos_idx: usize, price: f64) {
        self.close = Some((pos_idx, price));
    }
}

impl From<(usize, PositionSide, f64)> for PositionEvent {
    fn from((pos_idx, side, price): (usize, PositionSide, f64)) -> Self {
        Self {
            open: (pos_idx, side, price),
            close: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_event() {
        let position = Position::from((PositionSide::Long, 1.0, 1.0));
        let mut event = PositionEvent::from((1, position.side, position.entry_price));
        event.close(3, 2.0);
        assert_eq!(event.len(), Some(2));
    }
}
