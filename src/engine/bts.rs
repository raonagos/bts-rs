use chrono::DateTime;

use super::*;

fn get_data() -> Vec<Candle> {
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

    vec![candle]
}

fn get_long_data() -> Vec<Candle> {
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

    vec![candle1, candle2, candle3]
}

fn get_short_data() -> Vec<Candle> {
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

    vec![candle1, candle2, candle3]
}

fn get_long_data_trailing_stop() -> Vec<Candle> {
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

    vec![candle1, candle2, candle3, candle4]
}

fn get_long_data_trailing_stop_loss() -> Vec<Candle> {
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

    vec![candle1, candle2]
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
    bt.place_order(order.clone()).unwrap();

    assert!(!bt.orders.is_empty());
    assert_eq!(bt.balance(), 1000.0);
    assert_eq!(bt.total_balance(), 1000.0);
    assert_eq!(bt.free_balance().unwrap(), 890.0); // 890 with fees \ 900 without fees

    bt.delete_order(&order, true).unwrap();

    assert!(bt.orders.is_empty());
    assert_eq!(bt.balance(), 1000.0);
    assert_eq!(bt.total_balance(), 1000.0);
    assert_eq!(bt.free_balance().unwrap(), 1000.0);

    // Open long, take-profit
    {
        let data = get_long_data();
        let balance = 1000.0;
        let market_fee = 0.1; // 0.1%
        let mut bt = Backtest::new(data, balance, Some((market_fee, 0.01))).unwrap();

        let candle = bt.next().unwrap();
        let price = candle.close(); // 100
        let take_profit = OrderType::TakeProfitAndStopLoss(price.addpercent(20.0), 0.0);
        let order = Order::from((OrderType::Market(price), take_profit, 1.0, OrderSide::Buy));

        let open_fee = price * 1.0 * market_fee;
        let expected_total_cost = price + open_fee; // 100 + 0.10% = 110.0

        bt.place_order(order).unwrap();
        bt.execute_orders(&candle).unwrap();

        assert!(!bt.positions.is_empty());
        assert_eq!(bt.balance(), 890.0);
        assert_eq!(bt.total_balance(), 890.0);
        assert_eq!(bt.free_balance().unwrap(), 1000.0 - expected_total_cost);

        let candle = bt.next().unwrap();
        bt.execute_positions(&candle).unwrap(); // close = 110, p&l brut = +10
        assert!(!bt.positions.is_empty());

        let candle = bt.next().unwrap();
        bt.execute_positions(&candle).unwrap(); // close = 120, take profit

        assert!(bt.positions.is_empty());
        assert_eq!(bt.balance(), 1000.0); // balance = 1020 - (10 * 2) (fees)
        assert_eq!(bt.total_balance(), 1000.0);
        assert_eq!(bt.free_balance().unwrap(), 1000.0);
    }
}

#[test]
fn scenario_open_position_with_market_fees() {
    let data = get_long_data();
    let balance = 1000.0;
    let market_fee = 0.1; // 0.1%
    let mut bt = Backtest::new(data, balance, Some((market_fee, 0.01))).unwrap();

    let candle = bt.next().unwrap();
    let price = candle.close(); // 100
    let take_profit = OrderType::TakeProfitAndStopLoss(price.addpercent(20.0), 0.0);
    let order = Order::from((OrderType::Market(price), take_profit, 1.0, OrderSide::Buy));

    let open_fee = price * 1.0 * market_fee;
    let expected_total_cost = price + open_fee; // 100 + 0.10% = 110.0

    bt.place_order(order).unwrap();
    bt.execute_orders(&candle).unwrap();

    assert!(!bt.positions.is_empty());
    assert_eq!(bt.balance(), 890.0);
    assert_eq!(bt.total_balance(), 890.0);
    assert_eq!(bt.free_balance().unwrap(), 1000.0 - expected_total_cost);

    let candle = bt.next().unwrap();
    bt.execute_positions(&candle).unwrap(); // close = 110, p&l brut = +10
    assert!(!bt.positions.is_empty());

    let candle = bt.next().unwrap();
    bt.execute_positions(&candle).unwrap(); // close = 120, take profit

    assert!(bt.positions.is_empty());
    assert_eq!(bt.balance(), 1000.0); // balance = 1020 - (10 * 2) (fees)
    assert_eq!(bt.total_balance(), 1000.0);
    assert_eq!(bt.free_balance().unwrap(), 1000.0);
}

#[test]
fn scenario_place_and_delete_auto_a_market_order() {
    let data = get_data();
    let balance = 1000.0;
    let mut bt = Backtest::new(data, balance, None).unwrap();

    let candle = bt.next().unwrap();
    let price = candle.close(); // 110

    let order = Order::from((OrderType::Market(price * 3.0), 1.0, OrderSide::Buy));
    bt.place_order(order).unwrap(); // lock amount 110

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
    bt.place_order(order.clone()).unwrap(); // lock amount 110

    assert!(!bt.orders.is_empty());
    assert_eq!(bt.balance(), 1000.0);
    assert_eq!(bt.total_balance(), 1000.0);
    assert_eq!(bt.free_balance().unwrap(), 890.0);

    bt.delete_order(&order, true).unwrap(); // unlock amount 110

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
    bt.place_order(order).unwrap();

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
    bt.place_order(order).unwrap();

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
    bt.place_order(order).unwrap();

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
    bt.place_order(order).unwrap();

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
    bt.place_order(order).unwrap();
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
    bt.place_order(order).unwrap();
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
