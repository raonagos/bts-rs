use anyhow::Result;
use plotters::{element::PointCollection, prelude::*};

pub(crate) fn draw_lines<DB, CT>(
    chart: &mut ChartContext<DB, CT>,
    series: &[f64],
    color: &RGBColor,
) -> Result<()>
where
    DB: DrawingBackend,
    CT: CoordTranslate,
    for<'b> &'b DynElement<'static, DB, (i32, f64)>: PointCollection<'b, <CT>::From>,
    <DB>::ErrorType: 'static,
{
    let points = series.iter().enumerate().map(|(i, v)| (i as i32, *v));
    let series = LineSeries::new(points, color);
    chart.draw_series(series)?;
    Ok(())
}
