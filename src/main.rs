mod plot;
mod utils;

use crate::plot::*;
use crate::utils::*;

use anyhow::Result;
use plotters::prelude::*;
use plotters::style::full_palette::{BLACK, BLUE, GREEN, GREY_800, ORANGE, RED, WHITE};
use ta::{Close, High, Low, Open};

fn main() -> Result<()> {
    let candles = faker_candle(34);

    let opens = candles.iter().map(|c| c.open()).collect::<Vec<_>>();
    let highs = candles.iter().map(|c| c.high()).collect::<Vec<_>>();
    let lows = candles.iter().map(|c| c.low()).collect::<Vec<_>>();
    let closes = candles.iter().map(|c| c.close()).collect::<Vec<_>>();

    let root = SVGBackend::new("data/stock.svg", (2048, 1024)).into_drawing_area();
    root.fill(&BLACK)?;

    let rng = 0..candles.len() as i32;
    let from = closes
        .iter()
        .min_by(|a, b| a.partial_cmp(&b).unwrap())
        .copied()
        .ok_or(anyhow::Error::msg(""))?;
    let to = closes
        .iter()
        .max_by(|a, b| a.partial_cmp(&b).unwrap())
        .copied()
        .ok_or(anyhow::Error::msg(""))?;

    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(30)
        .y_label_area_size(50)
        .build_cartesian_2d(rng.clone(), from..to)?;
    chart
        .configure_mesh()
        .label_style(&WHITE)
        .bold_line_style(GREY_800)
        .draw()?;

    draw_lines(&mut chart, &opens, &GREEN)?;
    draw_lines(&mut chart, &highs, &RED)?;
    draw_lines(&mut chart, &lows, &BLUE)?;
    draw_lines(&mut chart, &closes, &ORANGE)?;

    Ok(())
}
