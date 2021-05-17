use crate::light_garden::*;

#[derive(Debug, PartialEq, Clone)]
pub struct StringMod {
    pub modulo: u64,
    pub num: u64,
    pub pow: u32,
    pub color: Color,
    pub turns: u64,
    pub init_curve: Curve,
    pub mode: StringModMode,
    pub modulo_colors: Vec<ModRemColor>,
    pub modulo_color_index: usize,
    pub nested: Option<Box<StringMod>>,
}

impl StringMod {
    pub fn new() -> Self {
        StringMod {
            modulo: 5,
            num: 1,
            pow: 0,
            color: [1.; 4],
            turns: 1,
            init_curve: Curve::Circle,
            mode: StringModMode::Mul,
            modulo_colors: Vec::new(),
            modulo_color_index: 0,
            nested: None,
        }
    }

    pub fn init_points(&self) -> Vec<P2> {
        let m = self.modulo;
        match self.init_curve {
            Curve::ComplexExp { c: complex } => {
                let points: Vec<P2> = (0..m)
                    .map(|n| {
                        let cp = complex.powu((self.turns * n) as u32);
                        P2::new(cp.re, cp.im)
                    })
                    .collect();
                points
            }
            Curve::Circle => {
                let points: Vec<P2> = (0..m)
                    .map(|n| {
                        let angle =
                            (self.turns * n) as Float * std::f64::consts::TAU / (m as Float);
                        let (y, x) = angle.sin_cos();
                        P2::new(x, y)
                    })
                    .collect();
                points
            }
            Curve::Hypotrochoid { r, s, d } => {
                let points: Vec<P2> = (0..m)
                    .map(|n| {
                        let angle =
                            (self.turns * n) as Float * std::f64::consts::TAU / (m as Float);
                        let small_radius = r as f64;
                        let big_radius = s as f64;
                        let smr = big_radius - small_radius;
                        let off_center = d as f64;
                        let ratio = smr + off_center;
                        let x = smr * angle.cos() + off_center * (angle * smr / small_radius).cos();
                        let y = smr * angle.sin() - off_center * (angle * smr / small_radius).sin();
                        P2::new(x, y) / ratio
                    })
                    .collect();
                points
            }
            Curve::Lissajous { a, b, delta } => {
                let points: Vec<P2> = (0..m)
                    .map(|n| {
                        let angle = (self.turns * n) as Float * TAU / (m as Float);
                        let x = (a as f64 * angle + delta).sin();
                        let y = (b as f64 * angle).sin();
                        P2::new(x, y)
                    })
                    .collect();
                points
            }
        }
    }

    pub fn line_crossings_as_points(&self) -> Vec<P2> {
        let draw_points = self.draw_init_points(self.init_points());
        let mut res = Vec::new();
        let mut lines = Vec::new();
        for w in draw_points.chunks_exact(2) {
            lines.push(LineSegment::from_ab(w[0].0, w[1].0));
        }
        for diff in 1..lines.len() {
            for ixa in 0..lines.len() {
                if let Some(p) = lines[ixa].intersect(&lines[(ixa + diff) % lines.len()]) {
                    res.push(p);
                }
            }
        }
        res
    }

    pub fn draw_init_points(&self, points: Vec<P2>) -> Vec<(P2, Color)> {
        let mut res = Vec::new();
        if points.is_empty() {
            return res;
        }
        let modulo = self.modulo;
        // we start at the index: 0
        for iix in 0..self.modulo {
            let ix = match self.mode {
                StringModMode::Add => ((iix + self.num) % modulo) as usize,
                StringModMode::Mul => ((iix * self.num) % modulo) as usize,
                StringModMode::Pow => (iix.pow(self.num as u32) % modulo) as usize,
                StringModMode::Base => (self.num.pow(iix as u32) % modulo) as usize,
            };
            res.push((points[iix as usize % points.len()], self.get_color(iix)));
            res.push((points[ix % points.len()], self.get_color(ix as u64)));
        }
        res
    }

    fn get_color(&self, ix: u64) -> Color {
        let mut color = [0.; 4];
        let mut num_colors = 0;
        for ModRemColor {
            modulo,
            rem,
            color: c,
        } in &self.modulo_colors
        {
            if (ix % modulo) == *rem {
                color[0] += c[0];
                color[1] += c[1];
                color[2] += c[2];
                color[3] += c[3];
                num_colors += 1;
            }
        }
        if num_colors == 0 {
            self.color
        } else {
            color[0] /= num_colors as f32;
            color[1] /= num_colors as f32;
            color[2] /= num_colors as f32;
            color[3] /= num_colors as f32;
            color
        }
    }

    pub fn draw(&self) -> Vec<(P2, Color)> {
        if let Some(string_mod) = self.nested.as_ref() {
            string_mod.draw_init_points(self.line_crossings_as_points())
        } else {
            self.draw_init_points(self.init_points())
        }
    }
}

impl Default for string_mod::StringMod {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct ModRemColor {
    pub modulo: u64,
    pub rem: u64,
    pub color: Color,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum StringModMode {
    Add,
    Mul,
    Pow,
    Base,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Curve {
    Circle,
    ComplexExp { c: na::Complex<f64> },
    Hypotrochoid { r: u64, s: u64, d: u64 },
    Lissajous { a: u64, b: u64, delta: f64 },
}
