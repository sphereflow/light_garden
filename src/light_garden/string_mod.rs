use crate::light_garden::*;

pub struct StringMod {
    pub on: bool,
    pub modulo: u64,
    pub num: u64,
    pub pow: u32,
    pub color: Color,
    pub init_complex: Option<na::Complex<f64>>,
    pub mode: StringModMode,
}

impl StringMod {
    pub fn new() -> Self {
        StringMod {
            on: false,
            modulo: 5,
            num: 1,
            pow: 0,
            color: [1.; 4],
            init_complex: None,
            mode: StringModMode::Mul,
        }
    }

    pub fn init_points(&self) -> Vec<P2> {
        let m = self.modulo;
        if let Some(complex) = self.init_complex {
            let points: Vec<P2> = (0..m)
                .map(|n| {
                    let cp = complex.powu(n as u32);
                    P2::new(cp.re, cp.im)
                })
                .collect();
            points
        } else {
            let points: Vec<P2> = (0..m)
                .map(|n| {
                    let angle = (n as Float) * std::f64::consts::TAU / (m as Float);
                    let (y, x) = angle.sin_cos();
                    P2::new(x, y)
                })
                .collect();
            points
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
            };
            res.push((points[iix as usize], self.color));
            res.push((points[ix], self.color));
        }
        res
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum StringModMode {
    Add,
    Mul,
    Pow,
}
