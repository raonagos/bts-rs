pub mod engine;
pub mod utils;

use std::ops::{Add, Div, Mul, Sub};

pub trait PercentCalculus<Rhs = Self> {
    fn addpercent(self, rhs: Rhs) -> Self;
    fn subpercent(self, rhs: Rhs) -> Self;
}

impl PercentCalculus for f64 {
    fn addpercent(self, percent: Self) -> Self {
        self.add(self.mul(percent.div(100.0)))
    }

    fn subpercent(self, percent: Self) -> Self {
        self.sub(self.mul(percent.div(100.0)))
    }
}
