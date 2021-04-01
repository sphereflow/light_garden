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
                        let r = r as f64;
                        let s = s as f64;
                        let smr = s - r;
                        let d = d as f64;
                        let ratio = smr + d;
                        let x = smr * angle.cos() + d * (angle * smr / r).cos();
                        let y = smr * angle.sin() - d * (angle * smr / r).sin();
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

    pub fn draw(&self, points: Vec<P2>) -> Vec<(P2, Color)> {
        let mut res = Vec::new();
        let m = self.modulo;
        // we start at the index: 0
        for iix in 1..self.modulo {
            let ix = match self.mode {
                StringModMode::Add => ((iix + self.num) % m) as usize,
                StringModMode::Mul => ((iix * self.num) % m) as usize,
                StringModMode::Pow => (iix.pow(self.num as u32) % m) as usize,
                StringModMode::Base => (self.num.pow(iix as u32) % m) as usize,
            };
            res.push((points[iix as usize], self.color));
            let mut color = [0.; 4];
            let mut num_colors = 0;
            for ModRemColor {
                modulo,
                rem,
                color: c,
            } in &self.modulo_colors
            {
                if (ix as u64 % modulo) == *rem {
                    color[0] += c[0];
                    color[1] += c[1];
                    color[2] += c[2];
                    color[3] += c[3];
                    num_colors += 1;
                }
            }
            if num_colors == 0 {
                res.push((points[ix], self.color));
            } else {
                color[0] /= num_colors as f32;
                color[1] /= num_colors as f32;
                color[2] /= num_colors as f32;
                color[3] /= num_colors as f32;
                res.push((points[ix], color));
            }
        }
        res
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
