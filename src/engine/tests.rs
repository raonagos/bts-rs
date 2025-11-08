use super::PositionSide::*;
use super::*;

fn get_data() -> Vec<Candle> {
    vec![
        Candle::from((100.0, 111.0, 99.0, 110.0, 1.0)),
        Candle::from((110.0, 112.0, 100.0, 120.0, 1.0)),
        Candle::from((120.0, 121.0, 100.0, 110.0, 1.0)),
    ]
}

#[test]
fn test_long_position() {
    let data = get_data();
    let balance = 1000.0;
    let mut bt = Backtest::new(data, balance);
    let mut _counter = 0;
    while let Some(candle) = bt.next() {
        let price = candle.close();
        if _counter == 0 {
            let result = bt.open_position((18, Long, price, 1.0).into()); // balance (1000.0) -= 110.0 * 1.0 => 890.0;
            assert!(result.is_ok());
            assert!(!bt.positions.is_empty());
            assert!(bt.balance < balance);
        }
        if _counter == 1 {
            let result = bt.close_position(18, price);
            assert!(result.is_ok());
            assert!(bt.positions.is_empty());
            assert!(bt.balance > balance);
        }
        _counter += 1;
    }
}

#[test]
fn test_short_position() {
    let data = get_data();
    let balance = 1000.0;
    let mut bt = Backtest::new(data, balance);
    let mut _counter = 0;
    while let Some(candle) = bt.next() {
        let price = candle.close();
        if _counter == 1 {
            let result = bt.open_position((18, Short, price, 1.0).into());
            assert!(result.is_ok());
            assert!(!bt.positions.is_empty());
            assert!(bt.balance > balance);
        }
        if _counter == 2 {
            let result = bt.close_position(18, price);
            assert!(result.is_ok());
            assert!(bt.positions.is_empty());
            assert!(bt.balance > balance);
        }
        _counter += 1;
    }
}

#[test]
fn test_failed_long_position() {
    let data = get_data();
    let balance = 1000.0;
    let mut bt = Backtest::new(data, balance);
    let mut _counter = 0;
    while let Some(candle) = bt.next() {
        let price = candle.close();
        if _counter == 1 {
            let result = bt.open_position((18, Long, price, 1.0).into()); // balance (1000.0) -= 110.0 * 1.0 => 890.0;
            assert!(result.is_ok());
            assert!(!bt.positions.is_empty());
            assert!(bt.balance < balance);
        }
        if _counter == 2 {
            let result = bt.close_position(18, price);
            assert!(result.is_ok());
            assert!(bt.positions.is_empty());
            assert!(bt.balance < balance);
        }
        _counter += 1;
    }
}

#[test]
fn test_failed_short_position() {
    let data = get_data();
    let balance = 1000.0;
    let mut bt = Backtest::new(data, balance);
    let mut _counter = 0;
    while let Some(candle) = bt.next() {
        let price = candle.close();
        if _counter == 0 {
            let result = bt.open_position((18, Short, price, 1.0).into());
            assert!(result.is_ok());
            assert!(!bt.positions.is_empty());
            assert!(bt.balance > balance);
        }
        if _counter == 1 {
            let result = bt.close_position(18, price);
            assert!(result.is_ok());
            assert!(bt.positions.is_empty());
            assert!(bt.balance < balance);
        }
        _counter += 1;
    }
}
