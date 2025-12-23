//! Module for visualizing backtest results and candle charts.
//!
//! It needs to enable `draws` feature to use it. Take a look at [trailing stop](https://github.com/raonagos/bts-rs/blob/master/examples/trailing_stop.rs#L70) for example.

use crate::engine::{Backtest, Candle};
use crate::errors::{Error, Result};
#[cfg(feature = "metrics")]
use crate::metrics::{Event, Metrics};

use charming::component::{Axis, DataZoom, DataZoomType, Grid, Title};
use charming::element::{AxisLabel, ItemStyle, Symbol, Tooltip, Trigger};
use charming::series::{Bar, Candlestick, Line, Scatter};
use charming::{Chart, HtmlRenderer};
use chrono::Duration;
use plotters::backend::{BitMapBackend, DrawingBackend, SVGBackend};
use plotters::coord::Shift;
use plotters::prelude::*;
use plotters::style::WHITE;
use plotters::style::full_palette::{LIME, ORANGE, PINK, PURPLE, TEAL};

/// Size of the X-axis.
const WIDTH: u32 = 1280;
/// Size of the Y-axis.
const HEIGHT: u32 = 900;
/// Size of the X-axis labels.
const X_LABEL_SIZE: i32 = 20;
/// Size of the Y-axis labels.
const Y_LABEL_SIZE: i32 = 20;

/// Output formats for the generated charts with output filename.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Default)]
pub enum DrawOutput {
    /// Save to the output SVG file.
    Svg(String),
    /// Save to the output PNG file.
    Png(String),
    /// Save to the output HTML file.
    Html(String),
    /// Print to the current console (not implemented).
    #[default]
    Inner,
}

/// Configuration options for chart generation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// Represents additional data series that can be plotted on a chart.
///
/// This enum is used to define custom visual elements (like technical indicators)
/// that can be overlaid on top of candlestick charts. Each variant corresponds to
/// a different type of visual representation:
///
/// - `Lines`: A continuous line series (e.g., RSI, MACD, moving averages)
/// - `Circles`: Discrete points marked as circles (e.g., divergence points, signals)
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Series {
    /// A continuous line series.
    ///
    /// Each value in the vector corresponds to a y-value at the same index
    /// as the candle in the chart's candle data. The x-coordinate is automatically
    /// derived from the candle's timestamp.
    Lines(Vec<f64>),
    /// A series of discrete points marked as circles.
    ///
    /// Each value in the vector corresponds to a y-value at the same index
    /// as the candle in the chart's candle data. The x-coordinate is automatically
    /// derived from the candle's timestamp.
    Circles(Vec<f64>),
}

/// Chart drawing utility for backtest visualization.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Draw {
    series: Vec<Series>,
    candles: Vec<Candle>,
    #[cfg(feature = "metrics")]
    metrics: Metrics,
    options: DrawOptions,
}

impl From<&Backtest> for Draw {
    fn from(value: &Backtest) -> Self {
        Self {
            series: Vec::new(),
            options: DrawOptions::default(),
            #[cfg(feature = "metrics")]
            metrics: Metrics::from(value),
            candles: value.candles().cloned().collect(),
        }
    }
}

impl Draw {
    /// Creates a new `Draw` instance.
    pub fn new(candles: Vec<Candle>, options: DrawOptions, #[cfg(feature = "metrics")] metrics: Metrics) -> Self {
        Self {
            candles,
            series: Vec::new(),
            #[cfg(feature = "metrics")]
            metrics,
            options,
        }
    }

    /// Sets the drawing options.
    pub fn with_options(mut self, options: DrawOptions) -> Self {
        self.options = options;
        self
    }

    /// Adds an additional data series to be plotted on the chart.
    ///
    /// This method allows you to overlay custom visual elements (like technical indicators)
    /// on top of the candlestick chart. The series will be drawn in the order they are added.
    ///
    /// ### Arguments
    ///
    /// * `series` - A `Series` enum variant containing the data to plot.
    ///   Can be either `Series::Lines` for continuous data or `Series::Circles` for discrete points.
    pub fn append_series(mut self, series: Series) -> Self {
        self.series.push(series);
        self
    }

    /// Generates and saves the chart based on the configured options.
    pub fn plot(&self) -> Result<()> {
        let candles = &self.candles;
        if candles.is_empty() {
            return Err(Error::CandleDataEmpty);
        }

        match &self.options.output {
            DrawOutput::Svg(path) => self.plot_svg(path),
            DrawOutput::Png(path) => self.plot_png(path),
            DrawOutput::Html(path) => self.plot_html(path),
            DrawOutput::Inner => self.plot_inner(),
        }
    }

    /// Saves the chart as an SVG file.
    fn plot_svg(&self, path: &str) -> Result<()> {
        let root = SVGBackend::new(path, (WIDTH, HEIGHT)).into_drawing_area();
        root.fill(&WHITE).map_err(|e| Error::Plotters(e.to_string()))?;
        self.draw_chart(&root)
    }

    /// Saves the chart as a PNG file.
    fn plot_png(&self, path: &str) -> Result<()> {
        let root = BitMapBackend::new(path, (WIDTH, HEIGHT)).into_drawing_area();
        root.fill(&WHITE).map_err(|e| Error::Plotters(e.to_string()))?;
        self.draw_chart(&root)
    }

    /// Saves the chart as an HTML file.
    fn plot_html(&self, path: &str) -> Result<()> {
        let chart = self.with_html_chart();
        let mut renderer = HtmlRenderer::new("BTS Chart", WIDTH.into(), HEIGHT.into());
        renderer.save(&chart, path)?;
        Ok(())
    }

    /// Displays the chart in the current console (not implemented).
    fn plot_inner(&self) -> Result<()> {
        Err(Error::Msg("Inner display is not implemented".to_string()))
    }

    /// Draws the main chart with price, volume, and metrics.
    fn draw_chart<DB: DrawingBackend>(&self, drawing_area: &DrawingArea<DB, Shift>) -> Result<()> {
        let total_height = drawing_area.dim_in_pixel().1 as f64;
        let mut volume_height = 0.0;
        if self.options.show_volume {
            volume_height = total_height * 0.2;
        }

        #[allow(unused_mut)]
        let mut metrics_height = 0.0;
        #[cfg(feature = "metrics")]
        if self.options.show_metrics {
            metrics_height = total_height * 0.2;
        }

        let price_height = total_height - volume_height - metrics_height;

        #[allow(unused_mut)]
        #[allow(unused_variables)]
        let (mut metrics_area, mut rest_area) = (drawing_area.clone(), drawing_area.clone());
        #[cfg(feature = "metrics")]
        if self.options.show_metrics {
            (metrics_area, rest_area) = drawing_area.split_vertically(metrics_height as u32)
        }

        let (price_area, volume_area) = if self.options.show_volume {
            rest_area.split_vertically(price_height as u32)
        } else {
            (rest_area.clone(), rest_area.clone())
        };

        // draw all charts
        self.draw_price_chart(&price_area)?;
        if self.options.show_volume {
            self.draw_volume_chart(&volume_area)?;
        }
        #[cfg(feature = "metrics")]
        if self.options.show_metrics {
            self.draw_metrics_chart(&metrics_area)?;
        }

        drawing_area.present().map_err(|e| Error::Plotters(e.to_string()))
    }

    /// Draws the price chart (candlesticks).
    fn draw_price_chart<DB: DrawingBackend>(&self, drawing_area: &DrawingArea<DB, Shift>) -> Result<()> {
        let min_price = self.candles.iter().map(|c| c.low()).fold(f64::INFINITY, f64::min);
        let max_price = self.candles.iter().map(|c| c.high()).fold(f64::NEG_INFINITY, f64::max);
        let first_time = self.candles.first().ok_or(Error::CandleNotFound)?.open_time();
        let last_time = self.candles.last().ok_or(Error::CandleNotFound)?.close_time();
        let price_range = max_price - min_price;
        let price_padding = price_range * 0.1;

        #[cfg(feature = "metrics")]
        let balances = self
            .metrics
            .events()
            .filter_map(|evt| match evt {
                Event::WalletUpdate { datetime, balance, .. } => Some((*datetime, *balance)),
                _ => None,
            })
            .collect::<Vec<_>>();

        #[cfg(not(feature = "metrics"))]
        let (min_balance, max_balance) = (0.0, 0.0);
        #[cfg(feature = "metrics")]
        let (min_balance, max_balance) = (
            balances.iter().map(|(_, b)| *b).fold(f64::INFINITY, f64::min),
            balances.iter().map(|(_, b)| *b).fold(f64::NEG_INFINITY, f64::max),
        );

        let (top, bottom) = if self.options.show_volume { (0, 0) } else { (10, 10) };
        let drawing_area = drawing_area.margin(top, bottom, 70, 70);
        let mut builder = ChartBuilder::on(&drawing_area);
        if !self.options.show_volume {
            builder.x_label_area_size(X_LABEL_SIZE);
        }

        #[cfg(not(feature = "metrics"))]
        {
            let title = self.options.title.as_deref().unwrap_or("BTS Chart");
            builder.caption(title, ("sans-serif", 30).into_font());
        }

        let mut chart = builder
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

        let candle_count = self.candles.len();

        let mut mesh = chart.configure_mesh();
        mesh.y_desc("Price")
            .y_label_style(("sans-serif", Y_LABEL_SIZE))
            .y_labels(5);

        if self.options.show_volume {
            mesh.disable_x_axis();
        } else {
            mesh.x_desc("Time")
                .x_label_style(("sans-serif", X_LABEL_SIZE))
                .x_labels(5);
        }

        mesh.draw().map_err(|e| Error::Plotters(e.to_string()))?;

        let candle_width = {
            let total_width = drawing_area.dim_in_pixel().0 as f64;
            let available_width = total_width - (X_LABEL_SIZE * 2) as f64;
            (available_width / candle_count as f64).max(5.0) as u32
        };

        chart
            .draw_series(self.candles.iter().map(|c| {
                let x = c.open_time();
                let open = c.open();
                let high = c.high();
                let low = c.low();
                let close = c.close();
                let color = if close >= open { GREEN.filled() } else { RED.filled() };
                CandleStick::new(x, open, high, low, close, color, color, candle_width)
            }))
            .map_err(|e| Error::Plotters(e.to_string()))?;

        if !self.series.is_empty() {
            let colors = [
                BLUE, GREEN, RED, CYAN, MAGENTA, YELLOW, BLACK, ORANGE, PURPLE, PINK, LIME, TEAL,
            ];
            let mut color_index = 0;

            self.series.iter().for_each(|s| {
                let color = colors[color_index % colors.len()];
                color_index += 1;

                match s {
                    Series::Lines(data) => {
                        let lines =
                            LineSeries::new(data.iter().zip(&self.candles).map(|(s, c)| (c.open_time(), *s)), color);
                        chart.draw_series(lines).expect("Draw line series");
                    }
                    Series::Circles(data) => {
                        let circles = data
                            .iter()
                            .zip(&self.candles)
                            .map(|(s, c)| Circle::new((c.open_time(), *s), 2.0, color));
                        chart.draw_series(circles).expect("Draw circle series");
                    }
                }
            });
        }

        #[cfg(feature = "metrics")]
        if self.options.show_metrics {
            use crate::PercentCalculus;

            let initial_balance = self.metrics.initial_balance();
            let red_balances = balances.iter().filter(|(_, balance)| *balance < initial_balance);
            let blue_balances = balances.iter().filter(|(_, balance)| *balance >= initial_balance);

            let opened_positions = self
                .metrics
                .events()
                .filter_map(|e| match e {
                    Event::AddPosition(date_time, position) => Some((date_time, position.entry_price())),
                    _ => None,
                })
                .map(|(datetime, price)| {
                    Circle::new(
                        (*datetime, price.expect("Invalid price").addpercent(5.0)),
                        2,
                        BLUE.filled(),
                    )
                });
            let closed_positions = self
                .metrics
                .events()
                .filter_map(|e| match e {
                    Event::DelPosition(date_time, position) => Some((date_time, position.entry_price())),
                    _ => None,
                })
                .map(|(datetime, price)| {
                    Circle::new(
                        (*datetime, price.expect("Invalid price").addpercent(5.0)),
                        2,
                        RED.filled(),
                    )
                });

            chart
                .draw_series(opened_positions)
                .map_err(|e| Error::Plotters(e.to_string()))?;
            chart
                .draw_series(closed_positions)
                .map_err(|e| Error::Plotters(e.to_string()))?;

            chart
                .draw_secondary_series(LineSeries::new(
                    blue_balances.map(|(datetime, balance)| (*datetime, *balance)),
                    BLUE,
                ))
                .map_err(|e| Error::Plotters(e.to_string()))?;

            chart
                .draw_secondary_series(LineSeries::new(
                    red_balances.map(|(datetime, balance)| (*datetime, *balance)),
                    RED,
                ))
                .map_err(|e| Error::Plotters(e.to_string()))?;
        }

        Ok(())
    }

    /// Draws the volume chart.
    fn draw_volume_chart<DB: DrawingBackend>(&self, drawing_area: &DrawingArea<DB, Shift>) -> Result<()> {
        let max_volume = self
            .candles
            .iter()
            .map(|c| c.volume())
            .fold(f64::NEG_INFINITY, f64::max);
        let volume_padding = max_volume * 0.1;
        let first_time = self.candles.first().ok_or(Error::CandleNotFound)?.open_time();
        let last_time = self.candles.last().ok_or(Error::CandleNotFound)?.close_time();
        let drawing_area = drawing_area.margin(0, 10, 70, 70);

        let mut chart = ChartBuilder::on(&drawing_area)
            .x_label_area_size(X_LABEL_SIZE)
            .y_label_area_size(Y_LABEL_SIZE)
            .build_cartesian_2d(first_time..last_time, 0.0..max_volume + volume_padding)
            .map_err(|e| Error::Plotters(e.to_string()))?;

        chart
            .configure_mesh()
            .x_desc("Time")
            .x_label_style(("sans-serif", X_LABEL_SIZE))
            .y_label_style(("sans-serif", Y_LABEL_SIZE))
            .x_labels(5)
            .y_labels(3)
            .draw()
            .map_err(|e| Error::Plotters(e.to_string()))?;

        chart
            .draw_series(self.candles.iter().map(|c| {
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
    fn draw_metrics_chart<DB: DrawingBackend>(&self, drawing_area: &DrawingArea<DB, Shift>) -> Result<()> {
        let title = self.options.title.as_deref().unwrap_or("BTS Chart");
        let max_drawdown = self.metrics.max_drawdown();
        let profit_factor = self.metrics.profit_factor();
        let sharpe_ratio = self.metrics.sharpe_ratio(0.0);
        let win_rate = self.metrics.win_rate();

        let drawing_area = drawing_area.margin(30, 0, 70, 70);
        let mut metrics_chart = ChartBuilder::on(&drawing_area)
            .caption(title, ("sans-serif", 30).into_font())
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

    /// Rendered html version.
    fn with_html_chart(&self) -> Chart {
        let min_value = self.candles.iter().map(|c| c.low()).fold(f64::INFINITY, f64::min);
        let max_value = self.candles.iter().map(|c| c.high()).fold(f64::NEG_INFINITY, f64::max);
        let title = self.options.title.as_deref().unwrap_or("BTS Chart");

        let mut chart = Chart::new()
            .title(Title::new().text(title).left("center"))
            .data_zoom(DataZoom::new().x_axis_index(vec![0, 1]).type_(DataZoomType::Slider))
            .grid(Grid::new().top("10%").height("50%"))
            .x_axis(
                Axis::new().grid_index(0).data(
                    self.candles
                        .iter()
                        .map(|c| c.open_time().date_naive().to_string())
                        .collect(),
                ),
            )
            .y_axis(
                Axis::new()
                    .grid_index(0)
                    .min((min_value * 0.95) as i64)
                    .max((max_value * 1.05) as i64)
                    .axis_label(AxisLabel::new()),
            )
            .series(
                Candlestick::new().data(
                    self.candles
                        .iter()
                        .enumerate()
                        .map(|(i, c)| {
                            let open = c.open();
                            let high = c.high();
                            let low = c.low();
                            let close = c.close();
                            vec![i as f64, open, high, low, close]
                        })
                        .collect(),
                ),
            );

        if self.options.show_volume {
            chart = chart
                .grid(Grid::new().top("65%").height("10%"))
                .x_axis(
                    Axis::new().grid_index(1).data(
                        self.candles
                            .iter()
                            .map(|c| c.open_time().date_naive().to_string())
                            .collect(),
                    ),
                )
                .y_axis(Axis::new().grid_index(1))
                .series(
                    Bar::new()
                        .x_axis_index(1)
                        .y_axis_index(1)
                        .data(self.candles.iter().map(|c| c.volume()).collect()),
                );
        }

        if !self.series.is_empty() {
            let colors = [
                "BLUE", "GREEN", "RED", "CYAN", "MAGENTA", "YELLOW", "BLACK", "ORANGE", "PURPLE", "PINK", "LIME",
                "TEAL",
            ];
            let mut color_index = 0;

            self.series.iter().for_each(|s| {
                let color = colors[color_index % colors.len()];
                color_index += 1;

                match s {
                    Series::Lines(data) => {
                        let lines = Line::new()
                            .x_axis_index(0)
                            .y_axis_index(0)
                            .data(data.to_vec())
                            .item_style(ItemStyle::new().color(color));

                        chart = chart.clone().series(lines);
                    }
                    Series::Circles(data) => {
                        let circles = Scatter::new()
                            .x_axis_index(0)
                            .y_axis_index(0)
                            .data(data.to_vec())
                            .symbol(Symbol::Circle)
                            .item_style(ItemStyle::new().color(color));

                        chart = chart.clone().series(circles);
                    }
                }
            });
        }

        chart.tooltip(Tooltip::new().trigger(Trigger::Axis))
    }
}
