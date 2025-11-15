use std::ops::Add;

/// Represents a trading wallet with balance and locked funds management.
#[derive(Debug)]
pub(crate) struct Wallet {
    // Initial balance used for reset
    initial_balance: f64,
    // Available balance
    balance: f64,
    // Funds locked in open positions
    locked: f64,
}

impl Wallet {
    /// Creates a new wallet with the given initial balance.
    /// Negative balances are set to 0.
    pub fn new(balance: f64) -> Self {
        Self {
            initial_balance: balance.max(0.0),
            balance: balance.max(0.0),
            locked: 0.0,
        }
    }

    /// Returns the free balance (available for new trades).
    pub fn free_balance(&self) -> f64 {
        self.balance - self.locked
    }

    pub fn balance(&self) -> f64 {
        self.balance
    }

    /// Adds funds to the wallet.
    pub fn add(&mut self, amount: f64, locked_amount: f64) {
        self.locked -= locked_amount.min(self.locked);
        self.balance += amount.add(locked_amount).max(0.0);
    }

    /// Subtracts funds from the balance (after an order is executed).
    /// Assumes funds are already locked.
    pub fn sub(&mut self, amount: f64) {
        self.balance -= amount.min(self.balance);
        self.locked -= amount.min(self.locked);
    }

    /// Locks additional funds for a position.
    pub fn lock(&mut self, amount: f64) -> bool {
        if self.free_balance() >= amount {
            self.locked += amount;
            true
        } else {
            false
        }
    }

    /// Unlocks funds when an order/position is closed.
    pub fn unlock(&mut self, amount: f64) {
        self.locked -= amount.min(self.locked);
    }

    /// Resets the wallet to its initial balance.
    pub fn reset(&mut self) {
        self.locked = 0.0;
        self.balance = self.initial_balance;
    }
}
