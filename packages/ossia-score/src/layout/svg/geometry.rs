use crate::AppError;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BBox {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl BBox {
    pub(super) fn points(points: &[(f64, f64)]) -> Result<Self, AppError> {
        if points.is_empty() || points.iter().any(|(x, y)| !x.is_finite() || !y.is_finite()) {
            return Err(AppError::invalid("invalid SVG coordinates"));
        }
        Ok(Self {
            x0: points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min),
            y0: points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min),
            x1: points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max),
            y1: points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max),
        })
    }

    pub(crate) fn center(self) -> (f64, f64) {
        ((self.x0 + self.x1) / 2., (self.y0 + self.y1) / 2.)
    }

    pub(crate) fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x0 && x <= self.x1 && y >= self.y0 && y <= self.y1
    }
}

#[derive(Clone, Copy)]
pub(super) struct Matrix(pub(super) [f64; 6]);

impl Matrix {
    pub(super) fn identity() -> Self {
        Self([1., 0., 0., 1., 0., 0.])
    }

    pub(super) fn point(self, x: f64, y: f64) -> (f64, f64) {
        let [a, b, c, d, e, f] = self.0;
        (a * x + c * y + e, b * x + d * y + f)
    }

    pub(super) fn then(self, r: Self) -> Self {
        let [a, b, c, d, e, f] = self.0;
        let [g, h, i, j, k, l] = r.0;
        Self([
            a * g + c * h,
            b * g + d * h,
            a * i + c * j,
            b * i + d * j,
            a * k + c * l + e,
            b * k + d * l + f,
        ])
    }

    pub(super) fn bbox(self, b: BBox) -> Result<BBox, AppError> {
        BBox::points(&[
            self.point(b.x0, b.y0),
            self.point(b.x1, b.y0),
            self.point(b.x0, b.y1),
            self.point(b.x1, b.y1),
        ])
    }

    pub(super) fn ellipse_bbox(self, cx: f64, cy: f64, rx: f64, ry: f64) -> Result<BBox, AppError> {
        let [a, b, c, d, ..] = self.0;
        let (x, y) = self.point(cx, cy);
        let x_extent = (a * rx).abs() + (c * ry).abs();
        let y_extent = (b * rx).abs() + (d * ry).abs();
        BBox::points(&[(x - x_extent, y - y_extent), (x + x_extent, y + y_extent)])
    }
}

fn number_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?").unwrap())
}

pub(super) fn numbers(mut s: &str) -> Result<Vec<f64>, AppError> {
    let mut out = Vec::new();
    loop {
        s = s.trim_start_matches(|c: char| c.is_ascii_whitespace() || c == ',');
        if s.is_empty() {
            return Ok(out);
        }
        let m = number_re()
            .find(s)
            .ok_or_else(|| AppError::invalid("invalid SVG number list"))?;
        let n = m
            .as_str()
            .parse::<f64>()
            .map_err(|_| AppError::invalid("invalid SVG number"))?;
        if !n.is_finite() {
            return Err(AppError::invalid("non-finite SVG number"));
        }
        out.push(n);
        s = &s[m.end()..];
    }
}

/// SVG `rotate(angle cx cy)`: degrees, about (cx, cy).
fn rotation(angle: f64, cx: f64, cy: f64) -> Matrix {
    let (sin, cos) = angle.to_radians().sin_cos();
    Matrix([
        cos,
        sin,
        -sin,
        cos,
        cx - cos * cx + sin * cy,
        cy - sin * cx - cos * cy,
    ])
}

pub(super) fn transform(mut s: &str) -> Result<Matrix, AppError> {
    let mut result = Matrix::identity();
    while !s.trim().is_empty() {
        s = s.trim_start_matches(|c: char| c.is_ascii_whitespace() || c == ',');
        if s.is_empty() {
            break;
        }
        let open = s
            .find('(')
            .ok_or_else(|| AppError::invalid("bad SVG transform"))?;
        let close = s
            .find(')')
            .ok_or_else(|| AppError::invalid("unclosed SVG transform"))?;
        if close < open {
            return Err(AppError::invalid("bad SVG transform parentheses"));
        }
        let n = numbers(&s[open + 1..close])?;
        let mat = match (s[..open].trim(), n.as_slice()) {
            ("translate", [x]) => Matrix([1., 0., 0., 1., *x, 0.]),
            ("translate", [x, y]) => Matrix([1., 0., 0., 1., *x, *y]),
            ("scale", [x]) => Matrix([*x, 0., 0., *x, 0., 0.]),
            ("scale", [x, y]) => Matrix([*x, 0., 0., *y, 0., 0.]),
            ("matrix", [a, b, c, d, e, f]) => Matrix([*a, *b, *c, *d, *e, *f]),
            // LilyPond rotates some markup and grobs, such as rotated text and arpeggio glyphs.
            ("rotate", [angle]) => rotation(*angle, 0., 0.),
            ("rotate", [angle, cx, cy]) => rotation(*angle, *cx, *cy),
            _ => return Err(AppError::unsupported("unsupported SVG transform")),
        };
        result = result.then(mat);
        s = &s[close + 1..];
    }
    Ok(result)
}

/// Conservative path bounds include Bézier control points. Unsupported commands never produce a guessed box.
pub fn path_bbox(mut d: &str) -> Result<BBox, AppError> {
    let mut points = Vec::new();
    let mut xy = (0., 0.);
    let mut start = xy;
    let mut cmd = None;
    let mut cubic_control: Option<(f64, f64)> = None;
    let mut quadratic_control: Option<(f64, f64)> = None;
    while !d.trim().is_empty() {
        d = d.trim_start_matches(|c: char| c.is_ascii_whitespace() || c == ',');
        if d.is_empty() {
            break;
        }
        let first = d.as_bytes()[0];
        if first.is_ascii_alphabetic() {
            cmd = Some(first as char);
            d = &d[1..];
            if first.eq_ignore_ascii_case(&b'z') {
                xy = start;
                points.push(xy);
                cmd = None;
                cubic_control = None;
                quadratic_control = None;
                continue;
            }
        }
        let c = cmd.ok_or_else(|| AppError::invalid("path coordinates without a command"))?;
        let count = match c.to_ascii_uppercase() {
            'M' | 'L' | 'T' => 2,
            'H' | 'V' => 1,
            'C' => 6,
            'S' | 'Q' => 4,
            _ => return Err(AppError::unsupported("unsupported SVG path command")),
        };
        let mut n = Vec::new();
        for _ in 0..count {
            d = d.trim_start_matches(|c: char| c.is_ascii_whitespace() || c == ',');
            let m = number_re()
                .find(d)
                .ok_or_else(|| AppError::invalid("incomplete SVG path coordinates"))?;
            let value = m
                .as_str()
                .parse::<f64>()
                .map_err(|_| AppError::invalid("bad SVG path number"))?;
            if !value.is_finite() {
                return Err(AppError::invalid("non-finite SVG path"));
            }
            n.push(value);
            d = &d[m.end()..];
        }
        let rel = c.is_ascii_lowercase();
        let base = if rel { xy } else { (0., 0.) };
        match c.to_ascii_uppercase() {
            'H' => {
                xy.0 = n[0] + if rel { xy.0 } else { 0. };
                points.push(xy);
            }
            'V' => {
                xy.1 = n[0] + if rel { xy.1 } else { 0. };
                points.push(xy);
            }
            'S' => {
                let control = cubic_control.map_or(xy, |p| (2. * xy.0 - p.0, 2. * xy.1 - p.1));
                points.push(control);
                for pair in n.as_chunks::<2>().0 {
                    points.push((pair[0] + base.0, pair[1] + base.1));
                }
                xy = *points.last().unwrap();
            }
            'T' => {
                let control = quadratic_control.map_or(xy, |p| (2. * xy.0 - p.0, 2. * xy.1 - p.1));
                points.push(control);
                points.push((n[0] + base.0, n[1] + base.1));
                xy = *points.last().unwrap();
                quadratic_control = Some(control);
            }
            _ => {
                for pair in n.as_chunks::<2>().0 {
                    points.push((pair[0] + base.0, pair[1] + base.1));
                }
                xy = *points.last().unwrap();
            }
        }
        cubic_control = match c.to_ascii_uppercase() {
            'C' => Some((n[2] + base.0, n[3] + base.1)),
            'S' => Some((n[0] + base.0, n[1] + base.1)),
            _ => None,
        };
        quadratic_control = match c.to_ascii_uppercase() {
            'Q' => Some((n[0] + base.0, n[1] + base.1)),
            'T' => quadratic_control,
            _ => None,
        };
        if c.eq_ignore_ascii_case(&'M') {
            start = xy;
            cmd = Some(if rel { 'l' } else { 'L' });
        }
    }
    BBox::points(&points)
}
