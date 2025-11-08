mod candle;
mod position;

#[cfg(test)]
mod tests;

pub use candle::*;
pub use position::*;

use crate::errors::{Error, Result};

#[derive(Debug)]
pub struct Backtest {
    data: Vec<Candle>,
    positions: Vec<Position>,
    // used to reset balance
    _balance: f64,
    balance: f64,
    index: usize,
    position_history: Vec<PositionEvent>,
}

impl Backtest {
    pub fn new(data: Vec<Candle>, initial_balance: f64) -> Self {
        Self {
            data,
            index: 0,
            positions: Vec::new(),
            balance: initial_balance,
            _balance: initial_balance,
            position_history: Vec::new(),
        }
    }

    pub fn current_balance(&self) -> f64 {
        self.balance
    }

    pub fn total_balance(&self, current_price: f64) -> f64 {
        let positions_value: f64 = self
            .positions
            .iter()
            .map(|p| match p.side() {
                PositionSide::Long => (current_price - p.entry_price()) * p.quantity(),
                PositionSide::Short => (p.entry_price() - current_price) * p.quantity(),
            })
            .sum();
        self.balance + positions_value
    }

    pub fn open_positions(&self) -> Vec<Position> {
        self.positions.clone()
    }

    pub fn position_history(&self) -> Vec<PositionEvent> {
        self.position_history.clone()
    }

    pub fn open_position(&mut self, position: Position) -> Result<()> {
        let (side, price, quantity) =
            (position.side(), position.entry_price(), position.quantity());
        let cost = price * quantity;

        if self.balance < cost {
            return Err(Error::LessBalance(cost));
        }

        match side {
            PositionSide::Long => self.balance -= cost,
            PositionSide::Short => self.balance += cost,
        }

        let position_evt = PositionEvent::from((self.index, price, PositionEventType::Open(side)));

        self.positions.push(position);
        self.position_history.push(position_evt);

        Ok(())
    }

    pub fn close_position(&mut self, position_id: u32, exit_price: f64) -> Result<f64> {
        if let Some(idx) = self.positions.iter().position(|p| p.id() == position_id) {
            let position = self.positions.remove(idx);
            let value = match position.side() {
                PositionSide::Long => {
                    let value = exit_price * position.quantity();
                    self.balance += value;
                    value
                }
                PositionSide::Short => {
                    self.balance -= position.entry_price() * position.quantity();
                    let profit = (position.entry_price() - exit_price) * position.quantity();
                    self.balance += profit;
                    profit
                }
            };

            self.position_history.push(PositionEvent::from((
                self.index,
                exit_price,
                PositionEventType::Close,
            )));
            return Ok(value);
        }

        Err(Error::EmptyPosition)
    }

    pub fn close_all_positions(&mut self, exit_price: f64) -> Result<f64> {
        let value = self
            .positions
            .iter()
            .map(|position| {
                let value = match position.side() {
                    PositionSide::Long => {
                        let value = exit_price * position.quantity();
                        self.balance += value;
                        value
                    }
                    PositionSide::Short => {
                        self.balance -= position.entry_price() * position.quantity();
                        let profit = (position.entry_price() - exit_price) * position.quantity();
                        self.balance += profit;
                        profit
                    }
                };

                self.position_history.push(PositionEvent::from((
                    self.index,
                    exit_price,
                    PositionEventType::Close,
                )));

                value
            })
            .sum();
        self.positions.clear();

        Ok(value)
    }

    pub fn reset(&mut self) {
        self.index = 0;
        self.positions = Vec::new();
        self.balance = self._balance;
        self.position_history = Vec::new();
    }
}

impl Iterator for Backtest {
    type Item = Candle;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.data.get(self.index).cloned();
        self.index += 1;
        item
    }
}
