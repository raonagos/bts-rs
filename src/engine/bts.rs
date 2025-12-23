use std::{
    collections::{VecDeque, vec_deque::Iter},
    sync::Arc,
};

#[cfg(feature = "metrics")]
use crate::metrics::*;
use crate::{
    PercentCalculus,
    engine::*,
    errors::{Error, Result},
};

#[cfg(test)]
impl Iterator for Backtest {
    type Item = Candle;

    fn next(&mut self) -> Option<Self::Item> {
        let candle = self.data.get(self.index).cloned();
        self.index += 1;
        candle
    }
}

/// Trait for aggregating candles based on different criteria.
pub trait Aggregation {
    /// Returns the aggregation factors (e.g., [1, 4, 8]).
    fn factors(&self) -> &[usize];

    /// Aggregates a set of candles into a single candle.
    fn aggregate(&self, candles: &[&Candle]) -> Result<Candle> {
        if candles.is_empty() {
            return Err(Error::CandleDataEmpty);
        }

        let first_candle = candles.first().ok_or(Error::CandleNotFound)?;
        let last_candle = candles.last().ok_or(Error::CandleNotFound)?;

        let open = first_candle.open();
        let close = last_candle.close();
        let uptrend_open = if open > close { open } else { close };
        let uptrend_close = if open > close { close } else { open };

        let high = candles.iter().map(|c| c.high()).fold(uptrend_open, f64::max);
        let low = candles.iter().map(|c| c.low()).fold(uptrend_close, f64::min);
        let volume = candles.iter().map(|c| c.volume()).sum::<f64>();
        let bid = candles.iter().map(|c| c.bid()).sum::<f64>();

        CandleBuilder::builder()
            .open(open)
            .high(high)
            .low(low)
            .close(close)
            .volume(volume)
            .bid(bid)
            .open_time(first_candle.open_time())
            .close_time(last_candle.close_time())
            .build()
    }

    /// Determines if the current set of candles should be aggregated.
    fn should_aggregate(&self, factor: usize, candles: &[&Candle]) -> bool {
        candles.len() == factor
    }
}

/// Backtesting engine for trading strategies.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct Backtest {
    #[cfg(test)]
    index: usize,
    wallet: Wallet,
    data: Arc<[Candle]>,
    #[cfg(feature = "metrics")]
    events: Vec<Event>,
    orders: VecDeque<Order>,
    positions: VecDeque<Position>,
    market_fees: Option<(f64, f64)>,
}

impl std::ops::Deref for Backtest {
    type Target = Wallet;

    fn deref(&self) -> &Self::Target {
        &self.wallet
    }
}

impl Backtest {
    /// Creates a new backtest instance.
    ///
    /// ### Arguments
    /// * `data` - Vector of candle data.
    /// * `initial_balance` - Initial wallet balance.
    /// * `market_fee` - Market *(market and limit)* fee percentage (e.g., 3 for 3%).
    ///   Fees are **only applied when positions are opened or closed**, not when orders are placed.
    ///
    /// ### Market Fees Behavior
    /// - **Order Placement**: No fees are charged when placing an order.
    ///   Fees are only deducted when the order is executed and a position is opened.
    /// - **Position Opening**: Fees are calculated as `price × quantity × market_fee`
    ///   and deducted from the wallet when the position is opened.
    /// - **Order Cancellation**: No fees are charged if an order is cancelled before execution.
    ///
    /// ### Returns
    /// The new backtest instance or an error.
    ///
    /// ### Example
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use bts_rs::prelude::*;
    /// use chrono::{DateTime, Duration};
    ///
    /// let candle = CandleBuilder::builder()
    ///     .open(100.0)
    ///     .high(110.0)
    ///     .low(95.0)
    ///     .close(105.0)
    ///     .volume(1.0)
    ///     .bid(0.5)
    ///     .open_time(DateTime::default())
    ///     .close_time(DateTime::default() + Duration::days(1))
    ///     .build()
    ///     .unwrap();
    ///
    /// let bts = Backtest::new(Arc::from_iter(vec![candle]), 1000.0, None).unwrap();
    /// // or with market fees (taker 3%, maker 1%)
    /// let bts = Backtest::new(Arc::from_iter(vec![candle]), 1000.0, Some((3.0, 1.0))).unwrap();
    /// ```
    pub fn new(data: Arc<[Candle]>, initial_balance: f64, market_fees: Option<(f64, f64)>) -> Result<Self> {
        if data.is_empty() {
            return Err(Error::CandleDataEmpty);
        }

        if let Some((market_fee, limit_fee)) = market_fees
            && (market_fee <= 0.0 || limit_fee <= 0.0)
        {
            return Err(Error::NegZeroFees);
        }

        let market_fees = market_fees.map(|(mf, lf)| (mf / 100.0, lf / 100.0));

        Ok(Self {
            data,
            #[cfg(test)]
            index: 0,
            market_fees,
            #[cfg(feature = "metrics")]
            events: Vec::new(),
            orders: VecDeque::new(),
            positions: VecDeque::new(),
            wallet: Wallet::new(initial_balance)?,
        })
    }

    /// Returns the market fees.
    pub fn market_fees(&self) -> Option<&(f64, f64)> {
        self.market_fees.as_ref()
    }

    /// Returns an iterator over the data.
    pub fn candles(&self) -> std::slice::Iter<'_, Candle> {
        self.data.iter()
    }

    /// Returns an iterator over the pending orders.
    pub fn orders(&self) -> Iter<'_, Order> {
        self.orders.iter()
    }

    /// Returns an iterator over the open positions.
    pub fn positions(&self) -> Iter<'_, Position> {
        self.positions.iter()
    }

    /// Returns an iterator over the recorded events.
    #[cfg(feature = "metrics")]
    pub fn events(&self) -> std::slice::Iter<'_, Event> {
        self.events.iter()
    }

    /// Places a new order.
    ///
    /// ### Arguments
    /// * `order` - The order to place.
    ///
    /// ### Returns
    /// Ok if successful, or an error.
    ///
    /// ### Example
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use bts_rs::prelude::*;
    /// use chrono::{DateTime, Duration};
    ///
    /// let candle = CandleBuilder::builder()
    ///     .open(100.0)
    ///     .high(110.0)
    ///     .low(95.0)
    ///     .close(105.0)
    ///     .volume(1.0)
    ///     .bid(0.5)
    ///     .open_time(DateTime::default())
    ///     .close_time(DateTime::default() + Duration::days(1))
    ///     .build()
    ///     .unwrap();
    ///
    /// let mut bts = Backtest::new(Arc::from_iter(vec![candle]), 1000.0, None).unwrap();
    /// let order = Order::from((OrderType::Limit(99.0), 1.0, OrderSide::Sell));
    /// bts.place_order(&candle, order).unwrap();
    /// ```
    pub fn place_order(&mut self, _candle: &Candle, order: Order) -> Result<()> {
        self.wallet.lock(order.cost()?)?;
        self.orders.push_back(order);
        #[cfg(feature = "metrics")]
        {
            let open_time = _candle.open_time();
            self.events.push(Event::from((open_time, &self.wallet)));
            self.events.push(Event::AddOrder(open_time, order));
        }
        Ok(())
    }

    /// Deletes a pending order.
    ///
    /// ### Arguments
    /// * `order` - Reference to the order to delete.
    ///
    /// ### Returns
    /// Ok if successful, or an error.
    ///
    /// ### Example
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use bts_rs::prelude::*;
    /// use chrono::{DateTime, Duration};
    ///
    /// let candle = CandleBuilder::builder()
    ///     .open(100.0)
    ///     .high(110.0)
    ///     .low(95.0)
    ///     .close(105.0)
    ///     .volume(1.0)
    ///     .bid(0.5)
    ///     .open_time(DateTime::default())
    ///     .close_time(DateTime::default() + Duration::days(1))
    ///     .build()
    ///     .unwrap();
    ///
    /// let mut bts = Backtest::new(Arc::from_iter(vec![candle]), 1000.0, None).unwrap();
    /// let order = Order::from((OrderType::Limit(99.0), 1.0, OrderSide::Sell));
    /// bts.place_order(&candle, order).unwrap();
    /// // if you call this function, always put `true` to delete
    /// bts.delete_order(&candle, &order, true).unwrap();
    /// ```
    pub fn delete_order(&mut self, _candle: &Candle, order: &Order, force_remove: bool) -> Result<()> {
        if force_remove {
            let order_idx = self
                .orders
                .iter()
                .position(|o| o == order)
                .ok_or(Error::OrderNotFound)?;
            self.orders.remove(order_idx).ok_or(Error::RemoveOrder)?;
        }
        self.wallet.unlock(order.cost()?)?;
        #[cfg(feature = "metrics")]
        {
            let open_time = _candle.open_time();
            self.events.push(Event::DelOrder(open_time, *order));
            self.events.push(Event::from((open_time, &self.wallet)));
        }
        Ok(())
    }

    /// Opens a new position.
    fn open_position(&mut self, _candle: &Candle, position: Position) -> Result<()> {
        self.wallet.sub(position.cost()?)?;
        if let Some((market_fee, limit_fee)) = self.market_fees {
            if position.is_market_type() {
                self.wallet.sub_fees(position.cost()? * market_fee)?;
            } else {
                self.wallet.sub_fees(position.cost()? * limit_fee)?;
            };
        }
        self.positions.push_back(position);
        #[cfg(feature = "metrics")]
        {
            let open_time = _candle.open_time();
            self.events.push(Event::from((open_time, &self.wallet)));
            self.events.push(Event::AddPosition(open_time, position));
        }
        Ok(())
    }

    /// Closes an existing position.
    ///
    /// ### Arguments
    /// * `position` - Reference to the position to close.
    /// * `exit_price` - The price at which to close the position.
    /// * `force_remove` - If true, removes the position without checking conditions.
    ///
    /// ### Returns
    /// The profit/loss from closing the position, or an error.
    ///
    /// ### Example
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use bts_rs::prelude::*;
    /// use chrono::{DateTime, Duration};
    ///
    /// let candle = CandleBuilder::builder()
    ///     .open(100.0)
    ///     .high(110.0)
    ///     .low(95.0)
    ///     .close(105.0)
    ///     .volume(1.0)
    ///     .bid(0.5)
    ///     .open_time(DateTime::default())
    ///     .close_time(DateTime::default() + Duration::days(1))
    ///     .build()
    ///     .unwrap();
    ///
    /// let mut bts = Backtest::new(Arc::from_iter(vec![candle]), 1000.0, None).unwrap();
    /// bts.run(|_bts, candle| {
    ///   let order = Order::from((OrderType::Limit(99.0), 1.0, OrderSide::Sell));
    ///   _bts.place_order(&candle, order).unwrap();
    ///   
    ///   let last_position = _bts.positions().last().cloned();
    ///   if let Some(position) = last_position {
    ///     // if you call this function, always put `true` to delete
    ///     _bts.close_position(candle, &position, 110.0, true).unwrap();
    ///   }
    ///
    ///   Ok(())
    /// }).unwrap();
    /// ```
    pub fn close_position(
        &mut self,
        _candle: &Candle,
        position: &Position,
        exit_price: f64,
        force_remove: bool,
    ) -> Result<f64> {
        if exit_price <= 0.0 || !exit_price.is_finite() {
            return Err(Error::ExitPrice(exit_price));
        }
        if force_remove {
            let pos_idx = self
                .positions
                .iter()
                .position(|p| p == position)
                .ok_or(Error::PositionNotFound)?;
            self.positions.remove(pos_idx).ok_or(Error::RemovePosition)?;
        }
        // Calculate profit/loss and update wallet
        let pnl = position.estimate_pnl(exit_price)?;
        let total_amount = pnl + position.cost()?;
        self.wallet.add(total_amount)?;
        self.wallet.sub_pnl(total_amount);
        if let Some((market_fee, limit_fee)) = self.market_fees {
            if position.is_market_type() {
                self.wallet.sub_fees(position.cost()? * market_fee)?;
            } else {
                self.wallet.sub_fees(position.cost()? * limit_fee)?;
            };
        }
        #[cfg(feature = "metrics")]
        {
            let mut _position = *position;
            _position.set_exit_price(exit_price)?;
            let open_time = _candle.open_time();
            self.events.push(Event::from((open_time, &self.wallet)));
            self.events.push(Event::DelPosition(open_time, _position));
        }
        Ok(pnl)
    }

    /// Closes all open positions at the given exit price.
    ///
    /// ### Arguments
    /// * `exit_price` - The price at which to close all positions.
    ///
    /// ### Returns
    /// Ok if successful, or an error.
    ///
    /// ### Example
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use bts_rs::prelude::*;
    /// use chrono::{DateTime, Duration};
    ///
    /// let candle = CandleBuilder::builder()
    ///     .open(100.0)
    ///     .high(110.0)
    ///     .low(95.0)
    ///     .close(105.0)
    ///     .volume(1.0)
    ///     .bid(0.5)
    ///     .open_time(DateTime::default())
    ///     .close_time(DateTime::default() + Duration::days(1))
    ///     .build()
    ///     .unwrap();
    ///
    /// let mut bts = Backtest::new(Arc::from_iter(vec![candle]), 1000.0, None).unwrap();
    /// bts.close_all_positions(&candle, 110.0).unwrap();
    /// ```
    pub fn close_all_positions(&mut self, candle: &Candle, exit_price: f64) -> Result<()> {
        while let Some(position) = self.positions.pop_front() {
            self.close_position(candle, &position, exit_price, false)?;
        }
        Ok(())
    }

    /// Executes pending orders based on current candle data.
    fn execute_orders(&mut self, candle: &Candle) -> Result<()> {
        let mut orders = VecDeque::with_capacity(self.orders.len());
        while let Some(order) = self.orders.pop_front() {
            let price = order.entry_price()?;
            if price >= candle.low() && price <= candle.high() {
                self.open_position(candle, Position::from(order))?;
            } else {
                //? if order is market type and does not between `high` and `low`, delete
                if order.is_market_type() {
                    self.delete_order(candle, &order, false)?;
                } else {
                    orders.push_back(order);
                }
            }
        }
        self.orders.append(&mut orders);
        Ok(())
    }

    /// Executes position management (take-profit, stop-loss, trailing stop).
    fn execute_positions(&mut self, candle: &Candle) -> Result<()> {
        let mut positions = VecDeque::with_capacity(self.positions.len());

        while let Some(mut position) = self.positions.pop_front() {
            let should_close = match position.exit_rule() {
                Some(OrderType::TakeProfitAndStopLoss(take_profit, stop_loss)) => {
                    if *take_profit < 0.0 || *stop_loss < 0.0 {
                        return Err(Error::NegTakeProfitAndStopLoss);
                    }

                    match position.side() {
                        PositionSide::Long => {
                            if *take_profit > 0.0 && take_profit <= &candle.high() {
                                Some(*take_profit)
                            } else if *stop_loss > 0.0 && stop_loss >= &candle.low() {
                                Some(*stop_loss)
                            } else {
                                None
                            }
                        }
                        PositionSide::Short => {
                            if *take_profit > 0.0 && take_profit >= &candle.low() {
                                Some(*take_profit)
                            } else if *stop_loss > 0.0 && stop_loss <= &candle.high() {
                                Some(*stop_loss)
                            } else {
                                None
                            }
                        }
                    }
                }
                Some(OrderType::TrailingStop(price, percent)) => {
                    if *price <= 0.0 || *percent <= 0.0 {
                        return Err(Error::NegZeroTrailingStop);
                    }

                    match position.side() {
                        PositionSide::Long => {
                            let execute_price = price.subpercent(*percent);
                            if execute_price >= candle.low() {
                                Some(execute_price)
                            } else {
                                if &candle.high() > price {
                                    position.set_trailingstop(candle.high());
                                }
                                None
                            }
                        }
                        PositionSide::Short => {
                            let execute_price = price.addpercent(*percent);
                            if execute_price <= candle.high() {
                                Some(execute_price)
                            } else {
                                if &candle.low() < price {
                                    position.set_trailingstop(candle.low());
                                }
                                None
                            }
                        }
                    }
                }
                None => None,
                _ => {
                    return Err(Error::MismatchedOrderType);
                }
            };

            match should_close {
                Some(exit_price) => {
                    self.close_position(candle, &position, exit_price, false)?;
                }
                None => positions.push_back(position),
            }
        }

        let mut total_unrealized_pnl = 0.0;
        for position in &positions {
            // calculate unrealized P&L for this position
            let current_price = candle.close();
            let pnl = position.estimate_pnl(current_price)?;
            total_unrealized_pnl += pnl;
        }

        self.positions.append(&mut positions);
        self.wallet.set_unrealized_pnl(total_unrealized_pnl);
        Ok(())
    }

    /// Runs the backtest, executing the provided function for each candle.
    ///
    /// ### Arguments
    /// * `strategy` - A closure that takes the backtest and current candle.
    ///
    /// ### Returns
    /// Ok if successful, or an error.
    ///
    /// ### Example
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use bts_rs::prelude::*;
    /// use chrono::{DateTime, Duration};
    ///
    /// let candle = CandleBuilder::builder()
    ///     .open(100.0)
    ///     .high(110.0)
    ///     .low(95.0)
    ///     .close(105.0)
    ///     .volume(1.0)
    ///     .bid(0.5)
    ///     .open_time(DateTime::default())
    ///     .close_time(DateTime::default() + Duration::days(1))
    ///     .build()
    ///     .unwrap();
    ///
    /// let mut bts = Backtest::new(Arc::from_iter(vec![candle]), 1000.0, None).unwrap();
    /// bts.run(|_bts, candle| {
    ///   let order = Order::from((OrderType::Limit(99.0), 1.0, OrderSide::Sell));
    ///   _bts.place_order(&candle, order)
    /// }).unwrap();
    /// ```
    pub fn run<S>(&mut self, mut strategy: S) -> Result<()>
    where
        S: FnMut(&mut Self, &Candle) -> Result<()>,
    {
        let candles = Arc::clone(&self.data);
        for candle in candles.iter() {
            strategy(self, candle)?;
            self.execute_orders(candle)?;
            self.execute_positions(candle)?;
        }
        Ok(())
    }

    /// Runs the backtest with aggregation, executing the provided function for each candle
    /// and its aggregated versions.
    ///
    /// ### Arguments
    /// * `aggregator` - An aggregator that defines how to group candles (e.g., by timeframe).
    /// * `strategy` - A closure that takes the backtest and a vector of candle references.
    ///
    /// The vector contains the current candle followed by any aggregated candles.
    ///
    /// ### Returns
    /// Ok if successful, or an error.
    ///
    /// ### Example
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use bts_rs::prelude::*;
    /// use chrono::{DateTime, Duration};
    ///
    /// let candle = CandleBuilder::builder()
    ///     .open(100.0)
    ///     .high(110.0)
    ///     .low(95.0)
    ///     .close(105.0)
    ///     .volume(1.0)
    ///     .bid(0.5)
    ///     .open_time(DateTime::default())
    ///     .close_time(DateTime::default() + Duration::days(1))
    ///     .build()
    ///     .unwrap();
    ///
    /// struct Aggregator;
    /// impl Aggregation for Aggregator {
    ///   fn factors(&self) -> &[usize] {
    ///     // return (1) the normal candle
    ///     &[1]
    ///   }
    /// }
    ///
    /// let mut bts = Backtest::new(Arc::from_iter(vec![candle]), 1000.0, None).unwrap();
    /// bts.run_with_aggregator(&Aggregator, |_bts, candles| {
    ///   let _candle = candles.last().unwrap();
    ///   Ok(())
    /// }).unwrap();
    /// ```
    pub fn run_with_aggregator<A, S>(&mut self, aggregator: &A, mut strategy: S) -> Result<()>
    where
        A: Aggregation,
        S: FnMut(&mut Self, Vec<&Candle>) -> Result<()>,
    {
        use std::collections::BTreeMap;

        let factors = aggregator.factors();
        if factors.is_empty() {
            return Err(Error::InvalidFactor);
        }

        let mut current_candles = BTreeMap::new();
        let mut aggregated_candles_map = BTreeMap::new();

        // Initialize the map with empty queues for each factor
        for &factor in factors {
            current_candles.insert(factor, VecDeque::with_capacity(factor));
            aggregated_candles_map.insert(factor, VecDeque::with_capacity(1));
        }

        let candles = Arc::clone(&self.data);
        for candle in candles.iter() {
            for (_, deque) in current_candles.iter_mut() {
                deque.push_back(candle);
            }

            for (factor, agg) in aggregated_candles_map.iter_mut() {
                let deque = current_candles.get_mut(factor).ok_or(Error::CandleDataEmpty)?;
                let contiguous_candles = deque.make_contiguous();
                if aggregator.should_aggregate(*factor, contiguous_candles) {
                    let candle = aggregator.aggregate(contiguous_candles)?;
                    agg.pop_front();
                    deque.pop_front();
                    agg.push_back(candle);
                }
            }

            let agg_candles = aggregated_candles_map.values().flatten().collect();
            strategy(self, agg_candles)?;
            self.execute_orders(candle)?;
            self.execute_positions(candle)?;
        }

        Ok(())
    }

    /// Resets the backtest to its initial state.
    pub fn reset(&mut self) {
        #[cfg(test)]
        {
            self.index = 0;
        }
        #[cfg(feature = "metrics")]
        {
            self.events = Vec::new();
        }

        self.wallet.reset();
        self.orders = VecDeque::new();
        self.positions = VecDeque::new();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::PercentCalculus;
    use crate::engine::*;

    use chrono::DateTime;

    fn get_data() -> Arc<[Candle]> {
        let candle = CandleBuilder::builder()
            .open(100.0)
            .high(111.0)
            .low(99.0)
            .close(110.0)
            .volume(1.0)
            .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
            .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
            .build()
            .unwrap();

        Arc::from_iter(vec![candle])
    }

    fn get_long_data() -> Arc<[Candle]> {
        let candle1 = CandleBuilder::builder()
            .open(90.0)
            .high(110.0)
            .low(80.0)
            .close(100.0)
            .volume(1.0)
            .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
            .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
            .build()
            .unwrap();
        let candle2 = CandleBuilder::builder()
            .open(100.0)
            .high(119.0)
            .low(90.0)
            .close(110.0)
            .volume(1.0)
            .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
            .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
            .build()
            .unwrap();
        let candle3 = CandleBuilder::builder()
            .open(110.0)
            .high(129.0)
            .low(100.0)
            .close(120.0)
            .volume(1.0)
            .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
            .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
            .build()
            .unwrap();

        let iter = vec![candle1, candle2, candle3];
        Arc::from_iter(iter)
    }

    fn get_short_data() -> Arc<[Candle]> {
        let candle1 = CandleBuilder::builder()
            .open(150.0)
            .high(160.0)
            .low(131.0)
            .close(140.0)
            .volume(1.0)
            .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
            .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
            .build()
            .unwrap();
        let candle2 = CandleBuilder::builder()
            .open(140.0)
            .high(150.0)
            .low(121.0)
            .close(130.0)
            .volume(1.0)
            .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
            .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
            .build()
            .unwrap();
        let candle3 = CandleBuilder::builder()
            .open(130.0)
            .high(140.0)
            .low(111.0)
            .close(120.0)
            .volume(1.0)
            .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
            .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
            .build()
            .unwrap();

        let iter = vec![candle1, candle2, candle3];
        Arc::from_iter(iter)
    }

    fn get_long_data_trailing_stop() -> Arc<[Candle]> {
        let candle1 = CandleBuilder::builder()
            .open(99.0)
            .high(101.0)
            .low(98.0)
            .close(100.0)
            .volume(1.0)
            .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
            .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
            .build()
            .unwrap();
        let candle2 = CandleBuilder::builder()
            .open(100.0)
            .high(110.0)
            .low(99.0)
            .close(108.0)
            .volume(1.0)
            .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
            .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
            .build()
            .unwrap();
        let candle3 = CandleBuilder::builder()
            .open(108.0)
            .high(140.0)
            .low(108.0)
            .close(135.0)
            .volume(1.0)
            .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
            .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
            .build()
            .unwrap();
        let candle4 = CandleBuilder::builder()
            .open(135.0)
            .high(139.9)
            .low(126.0)
            .close(130.0)
            .volume(1.0)
            .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
            .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
            .build()
            .unwrap();

        let iter = vec![candle1, candle2, candle3, candle4];
        Arc::from_iter(iter)
    }

    fn get_long_data_trailing_stop_loss() -> Arc<[Candle]> {
        let candle1 = CandleBuilder::builder()
            .open(99.0)
            .high(100.0)
            .low(98.0)
            .close(100.0)
            .volume(1.0)
            .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
            .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
            .build()
            .unwrap();
        let candle2 = CandleBuilder::builder()
            .open(100.0)
            .high(100.0)
            .low(90.0)
            .close(100.0)
            .volume(1.0)
            .open_time(DateTime::from_timestamp_secs(1515151515).unwrap())
            .close_time(DateTime::from_timestamp_secs(1515151516).unwrap())
            .build()
            .unwrap();

        let iter = vec![candle1, candle2];
        Arc::from_iter(iter)
    }

    #[test]
    fn scenario_place_and_delete_order_with_market_fees() {
        let data = get_data();
        let balance = 1000.0;
        let market_fee = 0.1; // 0.1%
        let mut bt = Backtest::new(data, balance, Some((market_fee, 0.01))).unwrap();
        let candle = bt.next().unwrap();
        let price = candle.close(); // 110

        let expected_fee = price * 1.0 * market_fee; // 110 * 1.0 * 0.001 = 0.11
        let _expected_total_cost = price + expected_fee; // 110 + 0.11 = 110.11

        let order = Order::from((OrderType::Market(price), 1.0, OrderSide::Buy));
        bt.place_order(&candle, order).unwrap();

        assert!(!bt.orders.is_empty());
        assert_eq!(bt.balance(), 1000.0);
        assert_eq!(bt.total_balance(), 1000.0);
        assert_eq!(bt.free_balance().unwrap(), 890.0); // 890 with fees \ 900 without fees

        bt.delete_order(&candle, &order, true).unwrap();

        assert!(bt.orders.is_empty());
        assert_eq!(bt.balance(), 1000.0);
        assert_eq!(bt.total_balance(), 1000.0);
        assert_eq!(bt.free_balance().unwrap(), 1000.0);

        // Open long, take-profit
        {
            let data = get_long_data();
            let balance = 1000.0;
            let market_fee = 1.0; // 1%
            let mut bt = Backtest::new(data, balance, Some((market_fee, 1.0))).unwrap();

            let candle = bt.next().unwrap();
            let price = candle.close(); // 100
            let take_profit = OrderType::TakeProfitAndStopLoss(price.addpercent(20.0), 0.0);
            let order = Order::from((OrderType::Market(price), take_profit, 1.0, OrderSide::Buy));

            let open_fee = price * 1.0 * (market_fee / 100.0);
            let expected_total_cost = price + open_fee; // 100 + 1.0% = 101.0

            bt.place_order(&candle, order).unwrap();
            bt.execute_orders(&candle).unwrap();

            assert!(!bt.positions.is_empty());
            assert_eq!(bt.balance(), 899.0);
            assert_eq!(bt.total_balance(), 899.0);
            assert_eq!(bt.free_balance().unwrap(), 1000.0 - expected_total_cost);

            let candle = bt.next().unwrap();
            bt.execute_positions(&candle).unwrap(); // close = 110, p&l brut = +10
            assert!(!bt.positions.is_empty());

            let candle = bt.next().unwrap();
            bt.execute_positions(&candle).unwrap(); // close = 120, take profit

            assert!(bt.positions.is_empty());
            assert_eq!(bt.balance(), 1018.0); // balance = 1020 - (1 * 2) (fees)
            assert_eq!(bt.total_balance(), 1018.0);
            assert_eq!(bt.free_balance().unwrap(), 1018.0);
        }
    }

    #[test]
    fn scenario_open_position_with_market_fees() {
        let data = get_long_data();
        let balance = 1000.0;
        let market_fee = 1.0; // 1%
        let mut bt = Backtest::new(data, balance, Some((market_fee, 1.0))).unwrap();

        let candle = bt.next().unwrap();
        let price = candle.close(); // 100
        let take_profit = OrderType::TakeProfitAndStopLoss(price.addpercent(20.0), 0.0);
        let order = Order::from((OrderType::Market(price), take_profit, 1.0, OrderSide::Buy));

        let open_fee = price * 1.0 * (market_fee / 100.0);
        let expected_total_cost = price + open_fee; // 100 + 1.0% = 101.0

        bt.place_order(&candle, order).unwrap();
        bt.execute_orders(&candle).unwrap();

        assert!(!bt.positions.is_empty());
        assert_eq!(bt.balance(), 899.0);
        assert_eq!(bt.total_balance(), 899.0);
        assert_eq!(bt.free_balance().unwrap(), 1000.0 - expected_total_cost);

        let candle = bt.next().unwrap();
        bt.execute_positions(&candle).unwrap(); // close = 110, p&l brut = +10
        assert!(!bt.positions.is_empty());

        let candle = bt.next().unwrap();
        bt.execute_positions(&candle).unwrap(); // close = 120, take profit

        assert!(bt.positions.is_empty());
        assert_eq!(bt.balance(), 1018.0); // balance = 1020 - (1 * 2) (fees)
        assert_eq!(bt.total_balance(), 1018.0);
        assert_eq!(bt.free_balance().unwrap(), 1018.0);
    }

    #[test]
    fn scenario_place_and_delete_auto_a_market_order() {
        let data = get_data();
        let balance = 1000.0;
        let mut bt = Backtest::new(data, balance, None).unwrap();

        let candle = bt.next().unwrap();
        let price = candle.close(); // 110

        let order = Order::from((OrderType::Market(price * 3.0), 1.0, OrderSide::Buy));
        bt.place_order(&candle, order).unwrap(); // lock amount 110

        assert_eq!(bt.balance(), 1000.0);
        assert_eq!(bt.total_balance(), 1000.0);
        assert_eq!(bt.free_balance().unwrap(), 670.0);

        bt.execute_orders(&candle).unwrap();

        assert!(bt.orders.is_empty());
        assert_eq!(bt.balance(), 1000.0);
        assert_eq!(bt.total_balance(), 1000.0);
        assert_eq!(bt.free_balance().unwrap(), 1000.0);
    }

    #[test]
    fn scenario_place_and_delete_order() {
        let data = get_data();
        let balance = 1000.0;
        let mut bt = Backtest::new(data, balance, None).unwrap();

        let candle = bt.next().unwrap();
        let price = candle.close(); // 110

        let order = Order::from((OrderType::Market(price), 1.0, OrderSide::Buy));
        bt.place_order(&candle, order).unwrap(); // lock amount 110

        assert!(!bt.orders.is_empty());
        assert_eq!(bt.balance(), 1000.0);
        assert_eq!(bt.total_balance(), 1000.0);
        assert_eq!(bt.free_balance().unwrap(), 890.0);

        bt.delete_order(&candle, &order, true).unwrap(); // unlock amount 110

        assert!(bt.orders.is_empty());
        assert_eq!(bt.balance(), 1000.0);
        assert_eq!(bt.total_balance(), 1000.0);
        assert_eq!(bt.free_balance().unwrap(), 1000.0);
    }

    #[test]
    fn scenario_open_long_position_and_take_profit() {
        let data = get_long_data();
        let balance = 1000.0;
        let mut bt = Backtest::new(data, balance, None).unwrap();

        let candle = bt.next().unwrap();
        let price = candle.close();

        let take_profit = OrderType::TakeProfitAndStopLoss(price.addpercent(20.0), 0.0);
        let order = Order::from((OrderType::Market(price), take_profit, 1.0, OrderSide::Buy));
        bt.place_order(&candle, order).unwrap();

        assert!(!bt.orders.is_empty());
        assert!(bt.positions.is_empty());
        assert_eq!(bt.balance(), 1000.0);
        assert_eq!(bt.total_balance(), 1000.0);
        assert_eq!(bt.free_balance().unwrap(), 900.0);

        bt.execute_orders(&candle).unwrap();

        assert!(bt.orders.is_empty());
        assert!(!bt.positions.is_empty());
        assert_eq!(bt.balance(), 900.0);
        assert_eq!(bt.total_balance(), 900.0);
        assert_eq!(bt.free_balance().unwrap(), 900.0);

        bt.execute_positions(&candle).unwrap();
        assert!(!bt.positions.is_empty());

        // next tick
        let candle = bt.next().unwrap();
        bt.execute_positions(&candle).unwrap(); // close = 110, p&l = +10

        assert!(!bt.positions.is_empty());
        assert_eq!(bt.balance(), 900.0);
        assert_eq!(bt.total_balance(), 910.0); // balance + p&l
        assert_eq!(bt.free_balance().unwrap(), 900.0);

        // next tick
        let candle = bt.next().unwrap();
        bt.execute_positions(&candle).unwrap(); // close = 120, take profit matched

        assert!(bt.positions.is_empty());
        assert_eq!(bt.balance(), 1020.0);
        assert_eq!(bt.total_balance(), 1020.0);
        assert_eq!(bt.free_balance().unwrap(), 1020.0);
    }

    #[test]
    fn scenario_open_long_position_and_stop_loss() {
        let data = get_short_data();
        let balance = 1000.0;
        let mut bt = Backtest::new(data, balance, None).unwrap();

        let candle = bt.next().unwrap();
        let price = candle.close();

        let stop_loss = OrderType::TakeProfitAndStopLoss(0.0, price - 20.0);
        let order = Order::from((OrderType::Market(price), stop_loss, 1.0, OrderSide::Buy));
        bt.place_order(&candle, order).unwrap();

        assert!(!bt.orders.is_empty());
        assert!(bt.positions.is_empty());
        assert_eq!(bt.balance(), 1000.0);
        assert_eq!(bt.total_balance(), 1000.0);
        assert_eq!(bt.free_balance().unwrap(), 860.0);

        bt.execute_orders(&candle).unwrap();

        assert!(bt.orders.is_empty());
        assert!(!bt.positions.is_empty());
        assert_eq!(bt.balance(), 860.0);
        assert_eq!(bt.total_balance(), 860.0);
        assert_eq!(bt.free_balance().unwrap(), 860.0);

        bt.execute_positions(&candle).unwrap();
        assert!(!bt.positions.is_empty());

        // next tick
        let candle = bt.next().unwrap();
        bt.execute_positions(&candle).unwrap(); // close = 130, p&l = -10

        assert!(!bt.positions.is_empty());
        assert_eq!(bt.balance(), 860.0);
        assert_eq!(bt.total_balance(), 850.0); // balance + p&l
        assert_eq!(bt.free_balance().unwrap(), 860.0);

        // next tick
        let candle = bt.next().unwrap();
        bt.execute_positions(&candle).unwrap(); // close = 120, stop loss matched

        assert!(bt.positions.is_empty());
        assert_eq!(bt.balance(), 980.0);
        assert_eq!(bt.total_balance(), 980.0);
        assert_eq!(bt.free_balance().unwrap(), 980.0);
    }

    #[test]
    fn scenario_open_short_position_and_take_profit() {
        let data = get_short_data();
        let balance = 1000.0;
        let mut bt = Backtest::new(data, balance, None).unwrap();

        let candle = bt.next().unwrap();
        let price = candle.close();

        let take_profit = OrderType::TakeProfitAndStopLoss(price - 20.0, 0.0);
        let order = Order::from((OrderType::Market(price), take_profit, 1.0, OrderSide::Sell));
        bt.place_order(&candle, order).unwrap();

        assert!(!bt.orders.is_empty());
        assert!(bt.positions.is_empty());
        assert_eq!(bt.balance(), 1000.0);
        assert_eq!(bt.total_balance(), 1000.0);
        assert_eq!(bt.free_balance().unwrap(), 860.0);

        bt.execute_orders(&candle).unwrap();

        assert!(bt.orders.is_empty());
        assert!(!bt.positions.is_empty());
        assert_eq!(bt.balance(), 860.0);
        assert_eq!(bt.total_balance(), 860.0);
        assert_eq!(bt.free_balance().unwrap(), 860.0);

        bt.execute_positions(&candle).unwrap();
        assert!(!bt.positions.is_empty());

        // next tick
        let candle = bt.next().unwrap();
        bt.execute_positions(&candle).unwrap(); // close = 130, p&l = +10

        assert!(!bt.positions.is_empty());
        assert_eq!(bt.balance(), 860.0);
        assert_eq!(bt.total_balance(), 870.0); // balance + p&l
        assert_eq!(bt.free_balance().unwrap(), 860.0);

        // next tick
        let candle = bt.next().unwrap();
        bt.execute_positions(&candle).unwrap(); // close = 120, take profit matched

        assert!(bt.positions.is_empty());
        assert_eq!(bt.balance(), 1020.0);
        assert_eq!(bt.total_balance(), 1020.0);
        assert_eq!(bt.free_balance().unwrap(), 1020.0);
    }

    #[test]
    fn scenario_open_short_position_and_stop_loss() {
        let data = get_long_data();
        let balance = 1000.0;
        let mut bt = Backtest::new(data, balance, None).unwrap();

        let candle = bt.next().unwrap();
        let price = candle.close();

        let stop_loss = OrderType::TakeProfitAndStopLoss(0.0, price.addpercent(20.0));
        let order = Order::from((OrderType::Market(price), stop_loss, 1.0, OrderSide::Sell));
        bt.place_order(&candle, order).unwrap();

        assert!(!bt.orders.is_empty());
        assert!(bt.positions.is_empty());
        assert_eq!(bt.balance(), 1000.0);
        assert_eq!(bt.total_balance(), 1000.0);
        assert_eq!(bt.free_balance().unwrap(), 900.0);

        bt.execute_orders(&candle).unwrap();

        assert!(bt.orders.is_empty());
        assert!(!bt.positions.is_empty());
        assert_eq!(bt.balance(), 900.0);
        assert_eq!(bt.total_balance(), 900.0);
        assert_eq!(bt.free_balance().unwrap(), 900.0);

        bt.execute_positions(&candle).unwrap();
        assert!(!bt.positions.is_empty());

        // next tick
        let candle = bt.next().unwrap();
        bt.execute_positions(&candle).unwrap(); // close = 110, p&l = -10

        assert!(!bt.positions.is_empty());
        assert_eq!(bt.balance(), 900.0);
        assert_eq!(bt.total_balance(), 890.0); // balance + p&l
        assert_eq!(bt.free_balance().unwrap(), 900.0);

        // next tick
        let candle = bt.next().unwrap();
        bt.execute_positions(&candle).unwrap(); // close = 120, stop loss matched

        assert!(bt.positions.is_empty());
        assert_eq!(bt.balance(), 980.0);
        assert_eq!(bt.total_balance(), 980.0);
        assert_eq!(bt.free_balance().unwrap(), 980.0);
    }

    #[test]
    fn scenario_open_long_position_with_trailing_stop_profit() {
        // enter at 100
        // the high is 140 and the trailing stop is set to 10%
        // exit at 126
        let data = get_long_data_trailing_stop();
        let balance = 1000.0;
        let mut bt = Backtest::new(data, balance, None).unwrap();

        let candle = bt.next().unwrap();
        let price = candle.close();

        let trailing_stop = OrderType::TrailingStop(price, 10.0);
        let order = Order::from((OrderType::Market(price), trailing_stop, 1.0, OrderSide::Buy));
        bt.place_order(&candle, order).unwrap();
        bt.execute_orders(&candle).unwrap();

        assert!(!bt.positions.is_empty());
        assert_eq!(bt.balance(), 900.0);
        assert_eq!(bt.total_balance(), 900.0);
        assert_eq!(bt.free_balance().unwrap(), 900.0);

        bt.execute_positions(&candle).unwrap();
        assert!(!bt.positions.is_empty());

        // next tick
        let candle = bt.next().unwrap();
        bt.execute_positions(&candle).unwrap();

        assert!(!bt.positions.is_empty());
        assert_eq!(bt.balance(), 900.0);
        assert_eq!(bt.total_balance(), 908.0);
        assert_eq!(bt.free_balance().unwrap(), 900.0);

        // next tick
        let candle = bt.next().unwrap();
        bt.execute_positions(&candle).unwrap();
        assert!(!bt.positions.is_empty());
        assert_eq!(bt.balance(), 900.0);
        assert_eq!(bt.total_balance(), 935.0);
        assert_eq!(bt.free_balance().unwrap(), 900.0);

        // next tick
        let candle = bt.next().unwrap();
        bt.execute_positions(&candle).unwrap();
        assert!(bt.positions.is_empty());
        assert_eq!(bt.balance(), 1026.0);
        assert_eq!(bt.total_balance(), 1026.0);
        assert_eq!(bt.free_balance().unwrap(), 1026.0);
    }

    #[test]
    fn scenario_open_long_position_with_trailing_stop_loss() {
        // enter at 100
        // the high is 100 and the trailing stop is set to 10%
        // exit at 90
        let data = get_long_data_trailing_stop_loss();
        let balance = 1000.0;
        let mut bt = Backtest::new(data, balance, None).unwrap();

        let candle = bt.next().unwrap();
        let price = candle.close();

        let trailing_stop = OrderType::TrailingStop(price, 10.0);
        let order = Order::from((OrderType::Market(price), trailing_stop, 1.0, OrderSide::Buy));
        bt.place_order(&candle, order).unwrap();
        bt.execute_orders(&candle).unwrap();

        assert!(!bt.positions.is_empty());
        assert_eq!(bt.balance(), 900.0);
        assert_eq!(bt.total_balance(), 900.0);
        assert_eq!(bt.free_balance().unwrap(), 900.0);

        bt.execute_positions(&candle).unwrap();
        assert!(!bt.positions.is_empty());

        // next tick
        let candle = bt.next().unwrap();
        bt.execute_positions(&candle).unwrap();

        assert!(bt.positions.is_empty());
        assert_eq!(bt.balance(), 990.0);
        assert_eq!(bt.total_balance(), 990.0);
        assert_eq!(bt.free_balance().unwrap(), 990.0);
    }

    struct TestAggregator;

    impl Aggregation for TestAggregator {
        fn factors(&self) -> &[usize] {
            &[1, 2]
        }
    }

    #[test]
    fn scenario_with_aggregator() {
        let data = get_short_data();
        let mut bt = Backtest::new(data, 1.0, None).unwrap();

        let mut ic = 0;
        let aggregator = TestAggregator;
        bt.run_with_aggregator(&aggregator, |_, candles| {
            let candle_one = candles.first();
            let candle_two = candles.get(1);

            // candle_two is none at ic = 0
            assert!(candle_one.is_some());

            if ic > 0 {
                assert!(candle_two.is_some());
                assert_ne!(candle_one, candle_two);
            }

            ic += 1;

            Ok(())
        })
        .unwrap();
    }
}
