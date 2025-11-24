#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::errors::{Error, Result};

/// Represents a trading wallet with balance and locked funds management.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct Wallet {
    // Initial balance used for reset
    initial_balance: f64,
    // Available balance
    balance: f64,
    // Funds locked in open positions
    locked: f64,
    // Unrealized profit/loss from open positions
    unrealized_pnl: f64,
    // Cumulative fees paid
    fees: f64,
}

impl Wallet {
    /// Creates a new wallet with the given initial balance.
    /// Negative balances are rejected.
    pub fn new(balance: f64) -> Result<Self> {
        if balance <= 0.0 {
            return Err(Error::NegZeroBalance(balance));
        }

        Ok(Self {
            balance,
            fees: 0.0,
            locked: 0.0,
            unrealized_pnl: 0.0,
            initial_balance: balance,
        })
    }

    #[cfg(feature = "metrics")]
    pub(crate) fn initial_balance(&self) -> f64 {
        self.initial_balance
    }

    #[cfg(feature = "metrics")]
    pub(crate) fn locked(&self) -> f64 {
        self.locked
    }

    #[cfg(feature = "metrics")]
    pub(crate) fn unrealized_pnl(&self) -> f64 {
        self.unrealized_pnl
    }

    /// Returns the balance.
    pub fn balance(&self) -> f64 {
        self.balance
    }

    /// Returns the total balance.
    pub fn total_balance(&self) -> f64 {
        self.balance + self.unrealized_pnl
    }

    /// Returns the free balance (available for new trades).
    pub fn free_balance(&self) -> Result<f64> {
        let free_balance = self.balance - self.locked;
        if free_balance < 0.0 {
            return Err(Error::NegFreeBalance(self.balance, self.locked));
        }
        Ok(free_balance)
    }

    /// Returns the fees paid to the market.
    pub fn fees_paid(&self) -> f64 {
        self.fees
    }

    /// Adds funds to the wallet.
    pub(crate) fn add(&mut self, amount: f64) -> Result<f64> {
        self.balance += amount;
        self.free_balance()
    }

    /// Subtracts funds from the balance (after an order is executed).
    /// Assumes funds are already locked.
    pub(crate) fn sub(&mut self, amount: f64) -> Result<f64> {
        self.balance -= amount;
        self.locked -= amount;
        self.free_balance()
    }

    /// Subtracts the market fees from the balance (after a position is executed).
    pub(crate) fn sub_fees(&mut self, amount: f64) -> Result<f64> {
        self.balance -= amount;
        self.fees += amount;
        self.free_balance()
    }

    /// Locks additional funds for a position.
    pub(crate) fn lock(&mut self, amount: f64) -> Result<()> {
        if amount <= 0.0 {
            return Err(Error::NegZeroBalance(amount));
        }
        let free_balance = self.free_balance()?;
        if free_balance < amount {
            return Err(Error::InsufficientFunds(amount, free_balance));
        }
        self.locked += amount;
        Ok(())
    }

    /// Unlocks funds when an order/position is closed.
    pub(crate) fn unlock(&mut self, amount: f64) -> Result<()> {
        if amount <= 0.0 {
            return Err(Error::NegZeroBalance(amount));
        }
        if self.locked - amount < 0.0 {
            return Err(Error::UnlockBalance(self.locked, amount));
        }
        self.locked -= amount;
        Ok(())
    }

    /// Updates the unrealized P&L.
    pub(crate) fn set_unrealized_pnl(&mut self, pnl: f64) {
        self.unrealized_pnl = pnl;
    }

    /// Subtracts the given amount from the wallet's unrealized P&L.
    ///
    /// This function is used when a position's unrealized P&L needs to be adjusted,
    /// typically when a position is closed and its P&L becomes realized.
    pub(crate) fn sub_pnl(&mut self, amount: f64) {
        self.unrealized_pnl -= amount;
    }

    /// Resets the wallet to its initial balance.
    pub(crate) fn reset(&mut self) {
        self.fees = 0.0;
        self.locked = 0.0;
        self.unrealized_pnl = 0.0;
        self.balance = self.initial_balance;
    }
}

#[cfg(test)]
#[test]
fn new_wallet_valid_balance() {
    let wallet = Wallet::new(100.0).unwrap();
    assert_eq!(wallet.balance(), 100.0);
    assert_eq!(wallet.free_balance().unwrap(), 100.0);
    assert_eq!(wallet.locked, 0.0);
}

#[cfg(test)]
#[test]
fn new_wallet_invalid_balance() {
    let result = Wallet::new(0.0);
    assert!(matches!(result, Err(Error::NegZeroBalance(_))));

    let result = Wallet::new(-10.0);
    assert!(matches!(result, Err(Error::NegZeroBalance(_))));
}

#[cfg(test)]
#[test]
fn unlock_funds_invalid() {
    let mut wallet = Wallet::new(100.0).unwrap();
    let result = wallet.unlock(20.0);
    assert!(matches!(result, Err(Error::UnlockBalance(_, _))));
}

#[cfg(test)]
#[test]
fn lock_and_unlock_funds() {
    let mut wallet = Wallet::new(100.0).unwrap();

    // Test lock
    wallet.lock(20.0).unwrap();
    assert_eq!(wallet.balance, 100.0);
    assert_eq!(wallet.locked, 20.0);

    // Test unlock
    wallet.unlock(20.0).unwrap();
    assert_eq!(wallet.balance, 100.0);
    assert_eq!(wallet.locked, 0.0);
}

#[cfg(test)]
#[test]
fn lock_insufficient_funds() {
    let mut wallet = Wallet::new(100.0).unwrap();
    let result = wallet.lock(150.0);
    assert!(matches!(result, Err(Error::InsufficientFunds(_, _))));
}

#[cfg(test)]
#[test]
fn lock_invalid_amount() {
    let mut wallet = Wallet::new(100.0).unwrap();
    let result = wallet.lock(-10.0);
    assert!(matches!(result, Err(Error::NegZeroBalance(_))));
}

#[cfg(test)]
#[test]
fn sub_funds() {
    let mut wallet = Wallet::new(100.0).unwrap();
    // place order
    wallet.lock(20.0).unwrap();

    // open position
    let free_balance = wallet.sub(20.0).unwrap();
    assert_eq!(free_balance, 80.0);
    assert_eq!(wallet.balance, 80.0);
    assert_eq!(wallet.locked, 0.0);
}

#[cfg(test)]
#[test]
fn add_funds() {
    let mut wallet = Wallet::new(100.0).unwrap();
    // close position
    let free_balance = wallet.add(50.0).unwrap();
    assert_eq!(free_balance, 150.0);
    assert_eq!(wallet.balance, 150.0);
    assert_eq!(wallet.locked, 0.0);
}

#[cfg(test)]
#[test]
fn reset_wallet() {
    let mut wallet = Wallet::new(100.0).unwrap();
    wallet.lock(20.0).unwrap();
    wallet.sub(20.0).unwrap();
    wallet.add(10.0).unwrap();
    wallet.sub_fees(0.2).unwrap();

    wallet.reset();
    assert_eq!(wallet.fees, 0.0);
    assert_eq!(wallet.locked, 0.0);
    assert_eq!(wallet.balance, 100.0);
    assert_eq!(wallet.total_balance(), 100.0);
    assert_eq!(wallet.free_balance().unwrap(), 100.0);
}

#[cfg(test)]
#[test]
fn open_close_profit_position() {
    let mut wallet = Wallet::new(100.0).unwrap();

    // place order
    wallet.lock(20.0).unwrap();
    assert_eq!(wallet.balance, 100.0);
    assert_eq!(wallet.locked, 20.0);
    assert_eq!(wallet.free_balance().unwrap(), 80.0);

    // open position
    wallet.sub(20.0).unwrap();
    assert_eq!(wallet.balance, 80.0);
    assert_eq!(wallet.locked, 0.0);
    assert_eq!(wallet.free_balance().unwrap(), 80.0);

    // close profitable position
    wallet.add(30.0).unwrap(); // 20.0 (initial locked) + 10.0 (profit)
    assert_eq!(wallet.balance, 110.0);
    assert_eq!(wallet.locked, 0.0);
    assert_eq!(wallet.free_balance().unwrap(), 110.0);
}

#[cfg(test)]
#[test]
fn open_close_loss_position() {
    let mut wallet = Wallet::new(100.0).unwrap();

    // place order
    wallet.lock(20.0).unwrap();
    assert_eq!(wallet.balance, 100.0);
    assert_eq!(wallet.locked, 20.0);
    assert_eq!(wallet.free_balance().unwrap(), 80.0);

    // open position
    wallet.sub(20.0).unwrap();
    assert_eq!(wallet.balance, 80.0);
    assert_eq!(wallet.locked, 0.0);
    assert_eq!(wallet.free_balance().unwrap(), 80.0);

    // close unprofitable position
    wallet.add(10.0).unwrap(); // 20.0 (initial locked) - 10.0 (loss)
    assert_eq!(wallet.balance, 90.0);
    assert_eq!(wallet.locked, 0.0);
    assert_eq!(wallet.free_balance().unwrap(), 90.0);
}

#[cfg(test)]
#[test]
fn unrealized_pnl() {
    let mut wallet = Wallet::new(100.0).unwrap();
    wallet.set_unrealized_pnl(10.0); // unrealized gain
    assert_eq!(wallet.unrealized_pnl, 10.0);
    assert_eq!(wallet.total_balance(), 110.0);
    assert_eq!(wallet.free_balance().unwrap(), 100.0);

    wallet.set_unrealized_pnl(-5.0); // unrealized loss
    assert_eq!(wallet.unrealized_pnl, -5.0);
    assert_eq!(wallet.total_balance(), 95.0);
    assert_eq!(wallet.free_balance().unwrap(), 100.0);
}
