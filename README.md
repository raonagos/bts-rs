# BTS: BackTest Strategy

**BTS** is a Rust library for backtesting trading strategies on candlestick data. It allows you to test technical indicators, custom strategies, and simulate trading performance on historical or generated data.

---

## **Features**

- **Technical Indicators**: Integrates popular indicators (Impulse MACD, Parabolic SAR, VWAP, etc.) and allows easy addition of new ones.
- **Backtesting**: Simulates trading strategies on historical or generated data.
- **Market Engine**: Processes candles one by one to test strategies in realistic conditions.
- **Performance Metrics**: Calculates P&L, drawdown, Sharpe ratio, and more.
- **Flexibility**: Compatible with the [`ta`](https://crates.io/crates/ta) crate for seamless integration with other indicators.

