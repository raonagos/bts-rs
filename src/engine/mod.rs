mod candle;
mod order;
mod position;

use crate::errors::{Error, Result};

pub use candle::*;
pub use order::*;
pub use position::*;

#[derive(Debug)]
pub struct Backtest {
    index: usize,
    data: Vec<Candle>,
    orders: Vec<Order>,
    positions: Vec<Position>,
    position_history: Vec<PositionEvent>,
}

impl Backtest {
    pub fn new(data: Vec<Candle>, _initial_balance: f64) -> Self {
        Self {
            data,
            index: 0,
            orders: Vec::new(),
            positions: Vec::new(),
            position_history: Vec::new(),
        }
    }

    pub fn place_order(&mut self, order: Order) -> Result<()> {
        self.orders.push(order);
        Ok(())
    }

    pub fn execute_orders(&mut self) {
        let current_candle = self.data.get(self.index).cloned();
        if let Some(cc) = current_candle {
            let mut i = 0;
            while i < self.orders.len() {
                let price = self.orders[i].entry_price();
                if price >= cc.low() && price <= cc.high() {
                    let order = self.orders.remove(i);
                    self.open_position(order.into());
                } else {
                    i += 1;
                }
            }
        }
    }

    fn open_position(&mut self, position: Position) {
        self.positions.push(position);
    }

    pub fn close_position(&mut self, position: &Position, _exit_price: f64) -> Result<()> {
        if let Some(pos_idx) = self.positions.iter().position(|p| p == position) {
            _ = self.positions.remove(pos_idx);
            return Ok(());
        }
        Err(Error::PositionNotFound)
    }

    pub fn reset(&mut self) {
        self.index = 0;
        self.positions = Vec::new();
        self.orders = Vec::new();
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
