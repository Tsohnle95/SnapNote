//! Normalized image annotations shared by preview and PNG export.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Tool {
    Move,
    Pen,
    Arrow,
    Rectangle,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mark {
    pub tool: Tool,
    pub points: Vec<[f32; 2]>,
}
impl Mark {
    pub fn segments(&self) -> Vec<([f32; 2], [f32; 2])> {
        let Some(&a) = self.points.first() else {
            return vec![];
        };
        let b = *self.points.last().unwrap();
        match self.tool {
            Tool::Move => vec![],
            Tool::Pen => {
                if self.points.len() == 1 {
                    vec![(a, a)]
                } else {
                    self.points.windows(2).map(|p| (p[0], p[1])).collect()
                }
            }
            Tool::Rectangle => {
                let c = [b[0], a[1]];
                let d = [a[0], b[1]];
                vec![(a, c), (c, b), (b, d), (d, a)]
            }
            Tool::Arrow => {
                let dx = b[0] - a[0];
                let dy = b[1] - a[1];
                let length = dx.hypot(dy);
                if length < 0.001 {
                    return vec![(a, b)];
                }
                let ux = dx / length;
                let uy = dy / length;
                let head = (length * 0.3).min(0.045);
                vec![
                    (a, b),
                    (
                        b,
                        [
                            b[0] - head * ux + head * 0.5 * uy,
                            b[1] - head * uy - head * 0.5 * ux,
                        ],
                    ),
                    (
                        b,
                        [
                            b[0] - head * ux - head * 0.5 * uy,
                            b[1] - head * uy + head * 0.5 * ux,
                        ],
                    ),
                ]
            }
        }
    }
}
pub fn render(image: &mut image::RgbaImage, marks: &[Mark]) {
    let w = image.width() as f32;
    let h = image.height() as f32;
    let radius = (w.min(h) * 0.006).max(1.);
    for mark in marks {
        for (a, b) in mark.segments() {
            let a = [a[0] * (w - 1.), a[1] * (h - 1.)];
            let b = [b[0] * (w - 1.), b[1] * (h - 1.)];
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let length = dx * dx + dy * dy;
            let left = (a[0].min(b[0]) - radius).max(0.) as u32;
            let right = (a[0].max(b[0]) + radius).min(w - 1.) as u32;
            let top = (a[1].min(b[1]) - radius).max(0.) as u32;
            let bottom = (a[1].max(b[1]) + radius).min(h - 1.) as u32;
            for y in top..=bottom {
                for x in left..=right {
                    let t = if length > 0. {
                        (((x as f32 - a[0]) * dx + (y as f32 - a[1]) * dy) / length).clamp(0., 1.)
                    } else {
                        0.
                    };
                    if (x as f32 - a[0] - t * dx).hypot(y as f32 - a[1] - t * dy) <= radius {
                        image.put_pixel(x, y, image::Rgba([255, 80, 80, 255]));
                    }
                }
            }
        }
    }
}
