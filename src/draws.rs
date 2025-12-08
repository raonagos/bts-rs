//! Module for visualizing backtest results and candle charts using the `plotters` library.

use crate::engine::{Backtest, Candle};
use crate::errors::{Error, Result};
#[cfg(feature = "metrics")]
use crate::metrics::{Event, Metrics};
use chrono::Duration;
use plotters::backend::{BitMapBackend, DrawingBackend, SVGBackend};
use plotters::coord::Shift;
use plotters::prelude::*;
use plotters::style::WHITE;

/// Aspect ratio for the generated charts.
const ASPECT_RATIO: f64 = 0.5625;
/// Size of the X-axis labels.
const X_LABEL_SIZE: i32 = 20;
/// Size of the Y-axis labels.
const Y_LABEL_SIZE: i32 = 20;

/// Output formats for the generated charts with output filename.
pub enum DrawOutput {
    /// Save to the output SVG file.
    Svg(&'static str),
    /// Save to the output PNG file.
    Png(&'static str),
    /// Save to the output HTML file (not implemented).
    Html(&'static str),
    /// Print to the current console (not implemented).
    Inner,
}

impl Default for DrawOutput {
    fn default() -> Self {
        Self::Inner
    }
}

/// Configuration options for chart generation.
#[derive(Default)]
pub struct DrawOptions {
    /// Chart title.
    title: Option<String>,
    /// Output format and path.
    output: DrawOutput,
    /// Whether to show the volume chart.
    show_volume: bool,
    #[cfg(feature = "metrics")]
    /// Whether to show the metrics chart.
    show_metrics: bool,
}

impl DrawOptions {
    /// Sets the chart title.
    pub fn title(mut self, title: impl ToString) -> Self {
        self.title = Some(title.to_string());
        self
    }

    /// Sets the output format and path.
    pub fn draw_output(mut self, output: DrawOutput) -> Self {
        self.output = output;
        self
    }

    /// Enables or disables the volume chart.
    pub fn show_volume(mut self, show: bool) -> Self {
        self.show_volume = show;
        self
    }

    #[cfg(feature = "metrics")]
    /// Enables or disables the metrics chart.
    pub fn show_metrics(mut self, show: bool) -> Self {
        self.show_metrics = show;
        self
    }
}

/// Chart drawing utility for backtest visualization.
#[derive(Default)]
pub struct Draw<'d> {
    /// Reference to the backtest data.
    backtest: Option<&'d Backtest>,
    /// Drawing options.
    options: DrawOptions,
}

impl<'d> Draw<'d> {
    /// Creates a new `Draw` instance with the given backtest.
    pub fn with_backtest(backtest: &'d Backtest) -> Self {
        Self {
            backtest: Some(backtest),
            options: DrawOptions::default(),
        }
    }

    /// Sets the drawing options.
    pub fn with_options(mut self, options: DrawOptions) -> Self {
        self.options = options;
        self
    }

    /// Generates and saves the chart based on the configured options.
    pub fn plot(&self) -> Result<()> {
        let backtest = self.backtest.ok_or(Error::Msg("No backtest provided".to_string()))?;
        let candles = backtest.candles().collect::<Vec<_>>();
        if candles.is_empty() {
            return Err(Error::CandleDataEmpty);
        }

        let title = self.options.title.as_deref().unwrap_or("BTS Chart");
        let mut height_factor = 1.0;
        if self.options.show_volume {
            height_factor += 0.4;
        }
        #[cfg(feature = "metrics")]
        if self.options.show_metrics {
            height_factor += 0.4;
        }

        let candle_count = candles.len() as u32;
        let width = 1280.max(10 * candle_count);
        let height = ((width as f64 * ASPECT_RATIO * height_factor) as u32).min(900);

        match self.options.output {
            DrawOutput::Svg(path) => self.plot_svg(path, &candles, width, height, title),
            DrawOutput::Png(path) => self.plot_png(path, &candles, width, height, title),
            DrawOutput::Html(path) => self.plot_html(path, &candles, width, height, title),
            DrawOutput::Inner => self.plot_inner(&candles, width, height, title),
        }
    }

    /// Saves the chart as an SVG file.
    fn plot_svg(&self, path: &str, candles: &[&Candle], width: u32, height: u32, title: &str) -> Result<()> {
        let backtest = self.backtest.ok_or(Error::Msg("No backtest provided".to_string()))?;
        let root = SVGBackend::new(path, (width, height)).into_drawing_area();
        root.fill(&WHITE).map_err(|e| Error::Plotters(e.to_string()))?;
        self.draw_chart(&root, candles, backtest, title)
    }

    /// Saves the chart as a PNG file.
    fn plot_png(&self, path: &str, candles: &[&Candle], width: u32, height: u32, title: &str) -> Result<()> {
        let backtest = self.backtest.ok_or(Error::Msg("No backtest provided".to_string()))?;
        let root = BitMapBackend::new(path, (width, height)).into_drawing_area();
        root.fill(&WHITE).map_err(|e| Error::Plotters(e.to_string()))?;
        self.draw_chart(&root, candles, backtest, title)
    }

    /// Saves the chart as an HTML file (not implemented).
    #[allow(unused_variables)]
    fn plot_html(&self, path: &str, candles: &[&Candle], width: u32, height: u32, title: &str) -> Result<()> {
        Err(Error::Msg("HTML output is not implemented".to_string()))
    }

    /// Displays the chart in the current console (not implemented).
    #[allow(unused_variables)]
    fn plot_inner(&self, candles: &[&Candle], width: u32, height: u32, title: &str) -> Result<()> {
        Err(Error::Msg("Inner display is not implemented".to_string()))
    }

    /// Draws the main chart with price, volume, and metrics.
    fn draw_chart<DB: DrawingBackend>(
        &self,
        drawing_area: &DrawingArea<DB, Shift>,
        candles: &[&Candle],
        backtest: &Backtest,
        title: &str,
    ) -> Result<()> {
        let total_height = drawing_area.dim_in_pixel().1 as f64;
        let volume_height = if self.options.show_volume {
            total_height * 0.2
        } else {
            0.0
        };

        #[cfg(not(feature = "metrics"))]
        let metrics_height = 0.0;
        #[cfg(feature = "metrics")]
        let metrics_height = if self.options.show_metrics {
            total_height * 0.2
        } else {
            0.0
        };

        let price_height = total_height - volume_height - metrics_height;

        #[cfg(not(feature = "metrics"))]
        let rest_area = drawing_area;
        #[cfg(feature = "metrics")]
        let (metrics_area, rest_area) = if self.options.show_metrics {
            drawing_area.split_vertically(metrics_height as u32)
        } else {
            (drawing_area.clone(), drawing_area.clone())
        };

        let (price_area, volume_area) = if self.options.show_volume {
            rest_area.split_vertically(price_height as u32)
        } else {
            (rest_area.clone(), rest_area.clone())
        };

        self.draw_price_chart(&price_area, backtest, title)?;
        if self.options.show_volume {
            self.draw_volume_chart(&volume_area, candles)?;
        }
        #[cfg(feature = "metrics")]
        if self.options.show_metrics {
            self.draw_metrics_chart(&metrics_area, backtest)?;
        }

        drawing_area.present().map_err(|e| Error::Plotters(e.to_string()))
    }

    /// Draws the price chart (candlesticks).
    fn draw_price_chart<DB: DrawingBackend>(
        &self,
        drawing_area: &DrawingArea<DB, Shift>,
        backtest: &Backtest,
        title: &str,
    ) -> Result<()> {
        let candles = backtest.candles().collect::<Vec<_>>();
        let min_price = candles.iter().map(|c| c.low()).fold(f64::INFINITY, f64::min);
        let max_price = candles.iter().map(|c| c.high()).fold(f64::NEG_INFINITY, f64::max);
        let first_time = candles.first().ok_or(Error::CandleNotFound)?.open_time();
        let last_time = candles.last().ok_or(Error::CandleNotFound)?.close_time();
        let price_range = max_price - min_price;
        let price_padding = price_range * 0.1;

        #[cfg(feature = "metrics")]
        let balances = backtest
            .events()
            .filter_map(|evt| match evt {
                Event::WalletUpdate {
                    timestamp,
                    balance, ..
                } => Some((*timestamp, *balance)),
                _ => None,
            })
            .fold(Vec::new(), |mut acc, (timestamp, balance)| {
                if !acc.iter().any(|(t, _)| t == &timestamp) {
                    acc.push((timestamp, balance));
                }
                acc
            });

        #[cfg(not(feature = "metrics"))]
        let (min_balance, max_balance) = (0.0, 0.0);
        #[cfg(feature = "metrics")]
        let (min_balance, max_balance) = {
            let min = balances.iter().map(|(_, b)| *b).fold(f64::INFINITY, f64::min);
            let max = balances.iter().map(|(_, b)| *b).fold(f64::NEG_INFINITY, f64::max);
            (min, max)
        };

        let (top, bottom) = if self.options.show_volume { (0, 0) } else { (10, 10) };
        let drawing_area = drawing_area.margin(top, bottom, 70, 70);
        let mut builder = ChartBuilder::on(&drawing_area);
        if !self.options.show_volume {
            builder.x_label_area_size(X_LABEL_SIZE);
        }

        let mut chart = builder
            .caption(title, ("sans-serif", 30).into_font())
            .y_label_area_size(Y_LABEL_SIZE)
            .right_y_label_area_size(Y_LABEL_SIZE)
            .build_cartesian_2d(
                first_time..last_time,
                min_price - price_padding..max_price + price_padding,
            )
            .map_err(|e| Error::Plotters(e.to_string()))?
            .set_secondary_coord(first_time..last_time, min_balance..max_balance);

        #[cfg(feature = "metrics")]
        if self.options.show_metrics {
            chart
                .configure_secondary_axes()
                .y_desc("Balance")
                .label_style(("sans-serif", Y_LABEL_SIZE))
                .y_labels(5)
                .draw()
                .map_err(|e| Error::Plotters(e.to_string()))?;
        }

        let candle_count = candles.len();
        let x_labels = candle_count / 15;

        {
            let mut mesh = chart.configure_mesh();
            mesh.y_desc("Price")
                .y_label_style(("sans-serif", Y_LABEL_SIZE))
                .y_labels(5);

            if self.options.show_volume {
                mesh.disable_x_axis();
            } else {
                mesh.x_desc("Time")
                    .x_label_style(("sans-serif", X_LABEL_SIZE))
                    .x_labels(x_labels);
            }

            mesh.draw().map_err(|e| Error::Plotters(e.to_string()))?;
        }

        let candle_width = {
            let total_width = drawing_area.dim_in_pixel().0 as f64;
            let available_width = total_width - (X_LABEL_SIZE * 2) as f64;
            let candles_count = candles.len() as f64;
            (available_width / candles_count).max(5.0) as u32
        };

        chart
            .draw_series(candles.iter().map(|c| {
                let x = c.open_time();
                let open = c.open();
                let high = c.high();
                let low = c.low();
                let close = c.close();
                let color = if close >= open { GREEN.filled() } else { RED.filled() };
                CandleStick::new(x, open, high, low, close, color, color, candle_width)
            }))
            .map_err(|e| Error::Plotters(e.to_string()))?;

        #[cfg(feature = "metrics")]
        if self.options.show_metrics {
            chart
                .draw_secondary_series(LineSeries::new(
                    balances.iter().map(|(t, b)| (*t, *b)),
                    RED,
                ))
                .map_err(|e| Error::Plotters(e.to_string()))?;
        }

        Ok(())
    }

    /// Draws the volume chart.
    fn draw_volume_chart<DB: DrawingBackend>(
        &self,
        drawing_area: &DrawingArea<DB, Shift>,
        candles: &[&Candle],
    ) -> Result<()> {
        let max_volume = candles.iter().map(|c| c.volume()).fold(f64::NEG_INFINITY, f64::max);
        let volume_padding = max_volume * 0.1;
        let first_time = candles.first().ok_or(Error::CandleNotFound)?.open_time();
        let last_time = candles.last().ok_or(Error::CandleNotFound)?.close_time();
        let drawing_area = drawing_area.margin(0, 10, 70, 70);

        let mut chart = ChartBuilder::on(&drawing_area)
            .x_label_area_size(X_LABEL_SIZE)
            .y_label_area_size(Y_LABEL_SIZE)
            .build_cartesian_2d(first_time..last_time, 0.0..max_volume + volume_padding)
            .map_err(|e| Error::Plotters(e.to_string()))?;

        let candle_count = candles.len();
        let x_labels = candle_count / 15;

        chart
            .configure_mesh()
            .x_desc("Time")
            .x_label_style(("sans-serif", X_LABEL_SIZE))
            .y_label_style(("sans-serif", Y_LABEL_SIZE))
            .x_labels(x_labels)
            .y_labels(3)
            .draw()
            .map_err(|e| Error::Plotters(e.to_string()))?;

        chart
            .draw_series(candles.iter().map(|c| {
                let x = c.open_time();
                let volume = c.volume();
                let color = if c.ask() >= c.bid() {
                    GREEN.mix(0.3)
                } else {
                    RED.mix(0.3)
                };
                Rectangle::new([(x, 0.0), (x + Duration::days(1), volume)], color.filled())
            }))
            .map(|_| ())
            .map_err(|e| Error::Plotters(e.to_string()))
    }

    /// Draws the metrics chart (if the "metrics" feature is enabled).
    #[cfg(feature = "metrics")]
    fn draw_metrics_chart<DB: DrawingBackend>(
        &self,
        drawing_area: &DrawingArea<DB, Shift>,
        backtest: &Backtest,
    ) -> Result<()> {
        let metrics = Metrics::from(backtest);
        let max_drawdown = metrics.max_drawdown();
        let profit_factor = metrics.profit_factor();
        let sharpe_ratio = metrics.sharpe_ratio(0.0);
        let win_rate = metrics.win_rate();

        let drawing_area = drawing_area.margin(30, 0, 70, 70);
        let mut metrics_chart = ChartBuilder::on(&drawing_area)
            .margin(20)
            .build_cartesian_2d(0.0..1.0, 0f64..100f64)
            .map_err(|e| Error::Plotters(e.to_string()))?;

        metrics_chart
            .configure_mesh()
            .disable_x_mesh()
            .disable_y_mesh()
            .draw()
            .map_err(|e| Error::Plotters(e.to_string()))?;

        let text = Text::new(
            format!(
                "Max Drawdown: {:.2}% | Profit Factor: {:.2} | Sharpe Ratio: {:.2} | Win Rate: {:.2}%",
                max_drawdown, profit_factor, sharpe_ratio, win_rate
            ),
            (0.0, 50.0),
            ("sans-serif", 28).into_font(),
        );

        metrics_chart
            .draw_series([text])
            .map(|_| ())
            .map_err(|e| Error::Plotters(e.to_string()))
    }
}
