use crate::errors::{Error, Result};

/// Represents a trading wallet with balance and locked funds management.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy)]
pub struct Wallet {
    fees: f64,
    locked: f64,
    balance: f64,
    unrealized_pnl: f64,
    positions_value: f64,
    initial_balance: f64,
}

impl Wallet {
    /// Creates a new wallet with the given initial balance.
    /// Negative balances are rejected.
    pub(crate) fn new(balance: f64) -> Result<Self> {
        if balance <= 0.0 {
            return Err(Error::NegZeroBalance(balance));
        }

        Ok(Self {
            balance,
            fees: 0.0,
            locked: 0.0,
            unrealized_pnl: 0.0,
            positions_value: 0.0,
            initial_balance: balance,
        })
    }

    /// Returns the initial balance.
    pub fn initial_balance(&self) -> f64 {
        self.initial_balance
    }

    /// Returns the locked balance.
    pub fn locked(&self) -> f64 {
        self.locked
    }

    /// Returns the unrealized pnl.
    pub fn unrealized_pnl(&self) -> f64 {
        self.unrealized_pnl
    }

    /// Returns the fees paid to the market.
    pub fn fees_paid(&self) -> f64 {
        self.fees
    }

    /// Returns the balance: the cash that is not invested in open positions.
    ///
    /// This is **not** the account value. A wallet holding a 900 position on a 1,000
    /// account has `balance() == 100` and [`total_balance()`](Self::total_balance) `1000`.
    pub fn balance(&self) -> f64 {
        self.balance
    }

    /// Returns the market value of the open positions (`cost + unrealized pnl`).
    pub fn positions_value(&self) -> f64 {
        self.positions_value
    }

    /// Returns the total balance, i.e. the mark-to-market account value:
    /// `balance() + positions_value()`.
    pub fn total_balance(&self) -> f64 {
        self.balance + self.positions_value
    }

    /// Returns the mark-to-market account value, an alias of
    /// [`total_balance()`](Self::total_balance).
    pub fn equity(&self) -> f64 {
        self.total_balance()
    }

    /// Returns the free balance (available for new trades).
    pub fn free_balance(&self) -> Result<f64> {
        let free_balance = self.balance - self.locked;
        if free_balance < 0.0 {
            return Err(Error::NegFreeBalance(self.balance, self.locked));
        }
        Ok(free_balance)
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

    /// Updates the market value of the open positions.
    pub(crate) fn set_positions_value(&mut self, value: f64) {
        self.positions_value = value;
    }

    /// Adds the cost of a newly opened position to the book.
    pub(crate) fn add_positions_value(&mut self, amount: f64) {
        self.positions_value += amount;
    }

    /// Removes a closed position (`cost + pnl`) from the book.
    pub(crate) fn sub_positions_value(&mut self, amount: f64) {
        self.positions_value -= amount;
    }

    /// Subtracts the given amount from the wallet's unrealized P&L.
    ///
    /// This function is used when a closed position's last mark has to be removed
    /// from the aggregate. Pass the position's P&L, **not** its cost: the cost lives
    /// in [`positions_value`](Self::positions_value) and is removed there.
    pub(crate) fn sub_pnl(&mut self, amount: f64) {
        self.unrealized_pnl -= amount;
    }

    /// Resets the wallet to its initial balance.
    pub(crate) fn reset(&mut self) {
        self.fees = 0.0;
        self.locked = 0.0;
        self.unrealized_pnl = 0.0;
        self.positions_value = 0.0;
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
    assert_eq!(wallet.total_balance(), 100.0); // unrealized pnl alone is not equity
    assert_eq!(wallet.free_balance().unwrap(), 100.0);

    wallet.set_unrealized_pnl(-5.0); // unrealized loss
    assert_eq!(wallet.unrealized_pnl, -5.0);
    assert_eq!(wallet.total_balance(), 100.0);
    assert_eq!(wallet.free_balance().unwrap(), 100.0);
}

#[cfg(test)]
#[test]
fn equity_is_cash_plus_positions_value() {
    let mut wallet = Wallet::new(1000.0).unwrap();

    // 900 notional on a 1,000 account: cash 100, position worth 900, equity 1,000
    wallet.lock(900.0).unwrap();
    wallet.sub(900.0).unwrap();
    wallet.add_positions_value(900.0);
    wallet.set_unrealized_pnl(90.0);
    wallet.set_positions_value(990.0); // marked at 110

    assert_eq!(wallet.balance(), 100.0);
    assert_eq!(wallet.positions_value(), 990.0);
    assert_eq!(wallet.total_balance(), 1090.0);
    assert_eq!(wallet.equity(), 1090.0);

    // closing the position at its mark leaves equity untouched
    wallet.add(990.0).unwrap();
    wallet.sub_positions_value(990.0);
    wallet.sub_pnl(90.0);
    assert_eq!(wallet.balance(), 1090.0);
    assert_eq!(wallet.total_balance(), 1090.0);
}
