use std::{f64::consts::FRAC_1_SQRT_2, path::PathBuf};

use clap::Parser;
use serde::Deserialize;
use svg::{
    node::element::{ClipPath, Definitions, Group, Line, Rectangle},
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
    let out_file = args
        .output
        .unwrap_or_else(|| args.input.with_extension("svg"));
    let grids: GridCollection = serde_yaml::from_reader(std::fs::File::open(&args.input)?)?;
    let doc = grids.to_svg();
    svg::write(std::fs::File::create(&out_file)?, &doc)?;
    Ok(())
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct GridCollection {
    /// Bounds for the generated SVG
    bounds: Rect,
    /// Bounds for the area of the image which will be rendered
    clip: Option<Rect>,
    /// Default stroke color
    stroke: Option<String>,
    /// Default stroke width
    stroke_width: Option<f64>,
    grids: Vec<Grid>,
}

impl GridCollection {
    pub fn to_svg(&self) -> svg::Document {
        let bounds = self.clip.unwrap_or(self.bounds);
        assert!(
            bounds.min_x < bounds.max_x,
            "min_x: {}, max_x: {}",
            bounds.min_x,
            bounds.max_x
        );
        assert!(
            bounds.min_y < bounds.max_y,
            "min_y: {}, max_y: {}",
            bounds.min_y,
            bounds.max_y
        );
        let document = svg::Document::new()
            .set(
                "viewBox",
                (
                    self.bounds.min_x,
                    self.bounds.min_y,
                    self.bounds.max_x - self.bounds.min_x,
                    self.bounds.max_y - self.bounds.min_y,
                ),
            )
            .add(
                Definitions::new().add(
                    ClipPath::new().set("id", "viewable-area").add(
                        Rectangle::new()
                            .set("x", bounds.min_x)
                            .set("y", bounds.min_y)
                            .set("width", bounds.max_x - bounds.min_x)
                            .set("height", bounds.max_y - bounds.min_y),
                    ),
                ),
            );
        let mut main_group = Group::new().set("stroke", self.stroke.as_deref().unwrap_or("black"));
        if let Some(width) = self.stroke_width {
            main_group.assign("stroke-width", width);
        }
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

            let mut group = Group::new().set("clip-path", "url(#viewable-area)");
            if let Some(stroke) = &grid.stroke {
                group.assign("stroke", &**stroke);
            }
            if let Some(width) = grid.stroke_width {
                group.assign("stroke-width", width);
            }
            if (45.0..135.0).contains(&theta) {
                // more horizontal than vertical
                assert!(sin >= FRAC_1_SQRT_2);
                let cot = cos / sin;
                // project onto the min_x line
                let y0 = cy + cot * (cx - bounds.min_x);
                // project onto the max_x line
                let y1 = cy + cot * (cx - bounds.max_x);
                let dy = (step / sin).abs();
                let min_idx = ((bounds.min_y - y0.max(y1)) / dy - 1.0) as i64;
                let max_idx = ((bounds.max_y - y0.min(y1)) / dy + 1.0) as i64;
                for i in min_idx..=max_idx {
                    group.append(
                        Line::new()
                            .set("x1", bounds.min_x)
                            .set("x2", bounds.max_x)
                            .set("y1", y0 + dy * (i as f64))
                            .set("y2", y1 + dy * (i as f64)),
                    );
                }
            } else {
                // more vertical than horizontal
                assert!(cos.abs() >= FRAC_1_SQRT_2);
                let tan = sin / cos;
                // project onto the min_y line
                let x0 = cx + tan * (cy - bounds.min_y);
                // project onto the max_y line
                let x1 = cx + tan * (cy - bounds.max_y);
                let dx = (step / cos).abs();
                let min_idx = ((bounds.min_x - x0.max(x1)) / dx - 1.0) as i64;
                let max_idx = ((bounds.max_x - x0.min(x1)) / dx + 1.0) as i64;
                for i in min_idx..=max_idx {
                    group.append(
                        Line::new()
                            .set("x1", x0 + dx * (i as f64))
                            .set("x2", x1 + dx * (i as f64))
                            .set("y1", bounds.min_y)
                            .set("y2", bounds.max_y),
                    );
                }
            }
            main_group.append(group);
        }
        document.add(main_group)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct Rect {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
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
