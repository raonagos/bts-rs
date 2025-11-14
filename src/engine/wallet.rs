use std::ops::{AddAssign, Sub};

#[derive(Debug)]
pub(crate) struct Wallet {
    // used to reset balance
    _balance: f64,
    pub balance: f64,
    locked: f64,
}

impl Wallet {
    pub fn new(balance: f64) -> Self {
        Self {
            _balance: balance.max(0.0),
            balance: balance.max(0.0),
            locked: 0.0,
        }
    }

    // pub(crate) fn total_balance(&self) -> f64 {
    //     self.balance + self.locked
    // }

    pub fn add(&mut self, rhs: f64) {
        self.locked = self.locked.sub(rhs).max(0.0);
        self.balance.add_assign(rhs);
    }

    pub fn sub(&mut self, rhs: f64) {
        self.balance = self.balance.sub(rhs).max(0.0);
        self.locked.add_assign(rhs);
    }

    pub fn reset(&mut self) {
        self.balance = self._balance;
        self.locked = 0.0;
    }
}
