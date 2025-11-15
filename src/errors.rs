use std::result::Result as StdResult;
use thiserror::Error as ThisError;

pub type Result<T> = StdResult<T, Error>;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("Insufficient funds {0}")]
    InsufficientFunds(f64),
    #[error("Position not found")]
    PositionNotFound,
    #[error("Order not found")]
    OrderNotFound,
    #[error("{0}")]
    IoError(#[from] std::io::Error),
    #[cfg(feature = "serde")]
    #[error("{0}")]
    JsonError(#[from] serde_json::Error),
}
