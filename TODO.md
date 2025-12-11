# BTS TODO

## ✅ Done
- [x] Wallet management
- [x] Position management
- [x] Order management system
- [x] Market simulation engine
- [x] Market fees implementation
- [x] Add `Candle` builder for validation
- [x] Add `Metrics` struct to wrap metrics (P&L, drawdown, Sharpe)
- [x] Timeframe/Volume aggregation (1H → 4H/8H/1D or 1D → 7D/1M)
- [x] Parameters optimization
- [x] Strategy examples (5+ templates)
- [x] WASM compilation support
- [x] Automated report generation (SVG/PNG)
- [x] Optimize (use Arc<[Candle]>, impl Copy trait) and remove unwrap and clone (partial)
- [x] Write better docs, examples ~~and tests~~

## 📌 In Progress

## 🚀 Road to v1.0.0

### Core Features

### Advanced Features
- [ ] Multi-strategy parallel execution
- [ ] Automated report generation (CONSOLE/HTML)
- [ ] Web/Desktop App dashboard integration
- [ ] Tracing and Progress when running strategy
- [ ] Add methods to modify orders/positions (update SL/TP/trailing stop)