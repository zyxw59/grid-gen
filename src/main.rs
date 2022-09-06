use std::{f64::consts::FRAC_1_SQRT_2, path::PathBuf};

use clap::Parser;
use serde::Deserialize;
use svg::{
    node::element::{Group, Line},
    Node,
};

#[derive(Debug, Parser)]
struct Args {
    /// Path of the input file
    input: PathBuf,
    /// Path of the output file
    output: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let out_file = args.output.unwrap_or_else(|| args.input.with_extension("svg"));
    let grids: GridCollection = serde_yaml::from_reader(std::fs::File::open(&args.input)?)?;
    let doc = grids.to_svg();
    svg::write(std::fs::File::create(&out_file)?, &doc)?;
    Ok(())
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct GridCollection {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    grids: Vec<Grid>,
}

impl GridCollection {
    pub fn to_svg(&self) -> svg::Document {
        assert!(self.min_x < self.max_x);
        assert!(self.min_y < self.max_y);
        let mut document = svg::Document::new().set(
            "viewBox",
            (
                self.min_x,
                self.min_y,
                self.max_x - self.min_x,
                self.max_y - self.min_y,
            ),
        );
        for grid in &self.grids {
            let mut theta = grid.theta.rem_euclid(360.0);
            let mut step = grid.step;
            assert_ne!(step, 0.0);
            if theta >= 180.0 {
                theta -= 180.0;
                step = -step;
            };
            let (cos, sin) = cos_sin_degrees(theta);
            let cx = grid.cx - cos * step * grid.center_position;
            let cy = grid.cy - sin * step * grid.center_position;

            let mut group = Group::new();
            if let Some(stroke) = &grid.stroke {
                group.assign("stroke", &**stroke);
            } else {
                group.assign("stroke", "black");
            }
            if let Some(width) = grid.stroke_width {
                group.assign("stroke-width", width);
            }
            if (45.0..135.0).contains(&theta) {
                // more horizontal than vertical
                assert!(sin >= FRAC_1_SQRT_2);
                let cot = cos / sin;
                // project onto the min_x line
                let y0 = cy + cot * (cx - self.min_x);
                // project onto the max_x line
                let y1 = cy + cot * (cx - self.max_x);
                let dy = (step / sin).abs();
                let min_idx = ((self.min_y - y0.max(y1)) / dy - 1.0) as i64;
                let max_idx = ((self.max_y - y0.min(y1)) / dy + 1.0) as i64;
                for i in min_idx..=max_idx {
                    group.append(
                        Line::new()
                            .set("x1", self.min_x)
                            .set("x2", self.max_x)
                            .set("y1", y0 + dy * (i as f64))
                            .set("y2", y1 + dy * (i as f64)),
                    );
                }
            } else {
                // more vertical than horizontal
                assert!(cos.abs() >= FRAC_1_SQRT_2);
                let tan = sin / cos;
                // project onto the min_y line
                let x0 = cx + tan * (cy - self.min_y);
                // project onto the max_y line
                let x1 = cx + tan * (cy - self.max_y);
                let dx = (step / cos).abs();
                let min_idx = ((self.min_x - x0.max(x1)) / dx - 1.0) as i64;
                let max_idx = ((self.max_x - x0.min(x1)) / dx + 1.0) as i64;
                for i in min_idx..=max_idx {
                    group.append(
                        Line::new()
                            .set("x1", x0 + dx * (i as f64))
                            .set("x2", x1 + dx * (i as f64))
                            .set("y1", self.min_y)
                            .set("y2", self.max_y),
                    );
                }
            }
            document.append(group);
        }
        document
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
struct Grid {
    /// x-coordinate of the center of the grid
    cx: f64,
    /// y-coordinate of the center of the grid
    cy: f64,
    /// Spacing between adjacent lines
    step: f64,
    /// Position of the center point relative to the grid. Integers mean the center point is on a
    /// grid line, non-integers mean it is somewhere between two grid lines.
    center_position: f64,
    /// Rotation clockwise from vertical about the center point, in degrees
    theta: f64,
    /// Stroke color
    stroke: Option<String>,
    /// Stroke width
    stroke_width: Option<f64>,
}

/// Returns the cos and sin of an angle in degrees, assuming it is in the range 0..180
fn cos_sin_degrees(theta: f64) -> (f64, f64) {
    assert!((0.0..180.0).contains(&theta));
    if theta == 0.0 {
        (1.0, 0.0)
    } else if theta == 45.0 {
        (FRAC_1_SQRT_2, FRAC_1_SQRT_2)
    } else if theta == 90.0 {
        (0.0, 1.0)
    } else if theta == 135.0 {
        (-FRAC_1_SQRT_2, FRAC_1_SQRT_2)
    } else {
        let rad = theta.to_radians();
        (rad.cos(), rad.sin())
    }
}
