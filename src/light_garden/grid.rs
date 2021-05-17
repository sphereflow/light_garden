use crate::light_garden::Color;
use collision2d::geo::*;

pub struct Grid {
    dist: f64,
    vertices: Vec<(P2, Color)>,
    bottom_left: P2,
    color: Color,
    pub on: bool,
    pub show: bool,
}

impl Grid {
    pub fn new(bounds: &Rect) -> Self {
        let mut res = Grid {
            dist: 0.1,
            vertices: Vec::new(),
            bottom_left: -P2::new(bounds.width, bounds.height) * 0.5,
            color: [1.; 4],
            on: false,
            show: false,
        };
        res.update_canvas_bounds(bounds);
        res
    }

    pub fn update_canvas_bounds(&mut self, bounds: &Rect) {
        self.vertices = vec![];
        let num_horizontal_lines: u64 = (bounds.height / self.dist) as u64;
        let num_vertical_lines: u64 = (bounds.width / self.dist) as u64;
        let [_right, bottom, left, _top] = bounds.line_segments();
        for i in 0..=num_horizontal_lines {
            let y_shift = i as f64 * -self.dist;
            let mut line = bottom.clone();
            line.shift(line.get_normal().into_inner() * y_shift);
            self.vertices.push((line.get_a(), self.color));
            self.vertices.push((line.get_b(), self.color));
        }
        for i in 0..=num_vertical_lines {
            let x_shift = i as f64 * -self.dist;
            let mut line = left.clone();
            line.shift(line.get_normal().into_inner() * x_shift);
            self.vertices.push((line.get_a(), self.color));
            self.vertices.push((line.get_b(), self.color));
        }
        self.bottom_left = -P2::new(bounds.width, bounds.height) * 0.5;
    }

    pub fn snap_to_grid(&self, pos: &mut P2) {
        if self.on {
            *pos -= self.bottom_left.coords;
            let mut mod_x = pos.x % self.dist;
            let mut mod_y = pos.y % self.dist;
            if mod_x > 0.5 * self.dist {
                mod_x -= self.dist;
            }
            if mod_y > 0.5 * self.dist {
                mod_y -= self.dist;
            }
            *pos -= V2::new(mod_x, mod_y);
            *pos += self.bottom_left.coords;
        }
    }

    pub fn get_render_lines<'a>(&'a self) -> Box<dyn Iterator<Item = &'a (P2, Color)> + 'a> {
        if self.show {
            Box::new(self.vertices.iter())
        } else {
            Box::new(std::iter::empty())
        }
    }

    pub fn get_dist(&self) -> f64 {
        self.dist
    }

    pub fn set_dist(&mut self, dist: f64, bounds: &Rect) {
        if self.dist != dist {
            self.dist = dist;
            self.update_canvas_bounds(bounds);
        }
    }

    pub fn get_color(&self) -> Color {
        self.color
    }
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
        for vertex in self.vertices.iter_mut() {
            vertex.1 = color;
        }
    }
}
