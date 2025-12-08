use bts_rs::engine::{Candle, CandleBuilder};
use bts_rs::PercentCalculus;
#[cfg(feature = "metrics")]
use bts_rs::metrics::Metrics;
use chrono::{Duration, Utc};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Generates deterministic candle data.
pub fn generate_sample_candles(count: usize, seed: u64, base_price: f64) -> Vec<Candle> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut open_time = Utc::now() - Duration::days(count as i64);
    let mut open = base_price;

    (0..count)
        .map(|_| {
            let close = open.addpercent(rng.random_range(-4.0..4.0));
            let high = open.max(close).addpercent(rng.random_range(0.0..3.0));
            let low = open.min(close).subpercent(rng.random_range(0.0..3.0));
            let volume = 1000.0.addpercent(rng.random_range(-15.0..15.0));
            let bid = volume * rng.random_range(0.33..0.77);

            let candle = CandleBuilder::builder()
                .open(open)
                .high(high)
                .low(low)
                .close(close)
                .volume(volume)
                .bid(bid)
                .open_time(open_time)
                .close_time(open_time + Duration::days(1))
                .build()
                .unwrap();

            open_time = candle.close_time();
            open = candle.close();

            candle
        })
        .collect()
}

pub fn example_candles() -> Vec<Candle> {
    generate_sample_candles(3000, 42, 100.0)
}

/// Pretty print Metrics
#[cfg(feature = "metrics")]
#[allow(dead_code)]
pub fn print_metrics(metrics: &Metrics, initial_balance: f64) {
    println!("=== Backtest Metrics ===");
    println!("Initial Balance: {:.2}", initial_balance);
    println!("Max Drawdown: {:.2}%", metrics.max_drawdown());
    println!("Profit Factor: {:.2}", metrics.profit_factor());
    println!("Sharpe Ratio (risk-free rate = 2%): {:.2}", metrics.sharpe_ratio(0.02));
    println!("Win Rate: {:.2}%", metrics.win_rate());
}

#[macro_export]
/// Pause and resume when press any key.
macro_rules! pause {
    () => {
        println!("[{}:{}] Pausing! Press enter to continue...", file!(), line!());
        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer).expect("Failed to read line");
    };
}

#[allow(dead_code)]
fn main() {}
