use crate::light_garden::*;

pub struct Drawer {
    lines: Vec<(P2, Color)>,
    triangles: Vec<(P2, Color)>,
    /// point size in relation to canvas size
    point_size: Float,
    canvas_bounds: Rect,
}

impl Drawer {
    pub fn new(canvas_bounds: &Rect) -> Self {
        Drawer {
            lines: Vec::new(),
            triangles: Vec::new(),
            point_size: 0.005,
            canvas_bounds: *canvas_bounds,
        }
    }

    pub fn draw_geo<G: Into<Geo>>(&mut self, into_geo: G, color: Color) {
        let geo: Geo = into_geo.into();
        match geo {
            Geo::GeoRect(r) => {
                for line in r.line_segments() {
                    self.draw_line_segment(&line, color);
                }
            }
            Geo::GeoCircle(c) => {
                let num_line_segments = 360;
                let r = c.radius;
                let o = c.get_origin();
                let mut points = Vec::new();
                for a in 0_u16..num_line_segments {
                    let angle = a as f64 * TAU / (num_line_segments as f64);
                    points.push((o + r * V2::new(angle.sin(), angle.cos()), color));
                }
                points.push((o + r * V2::new(0.0, 1.0), color));
                for w in points.windows(2) {
                    self.lines.push(w[0]);
                    self.lines.push(w[1]);
                }
            }
            Geo::GeoRay(r) => {
                if let Some(oot) = r.intersect(&self.canvas_bounds) {
                    self.lines.push((r.get_origin(), color));
                    self.lines.push((oot.get_first().0, color));
                }
            }
            Geo::GeoPoint(p) => self.draw_point(&p, color),
            Geo::GeoLogic(l) => {
                let a = l.get_a();
                let b = l.get_b();
                self.draw_geo(a, color);
                self.draw_geo(b, color);
            }
            Geo::GeoMCircle(mc) => {
                let ca = mc.circle_a();
                let cb = mc.circle_b();
                self.draw_geo(ca, color);
                self.draw_geo(cb, color);
            }
            Geo::GeoLineSegment(ls) => {
                self.lines.push((ls.get_a(), color));
                self.lines.push((ls.get_b(), color));
            }
            Geo::GeoCubicBezier(cb) => {
                let mut points = Vec::new();
                let num_line_segments = 400;
                for ti in 0..=num_line_segments {
                    let t = ti as f64 / (num_line_segments as f64);
                    points.push((cb.eval_at_t(t), color));
                }
                for w in points.windows(2) {
                    self.lines.push(w[0]);
                    self.lines.push(w[1]);
                }
            }
            Geo::GeoConvexPolygon(cp) => {
                for ls in cp.get_line_segments() {
                    self.draw_line_segment(&ls, color);
                }
            }
        }
    }

    pub fn draw_aabb(&mut self, aabb: &Aabb, color: Color) {
        self.draw_geo(aabb.to_rect(), color);
    }

    pub fn draw_line_segment(&mut self, ls: &LineSegment, color: Color) {
        self.lines.push((ls.get_a(), color));
        self.lines.push((ls.get_b(), color));
    }

    pub fn draw_point(&mut self, p: &P2, color: Color) {
        let width = self.canvas_bounds.width * self.point_size;
        let height = self.canvas_bounds.height * self.point_size;
        let aabb = Aabb {
            origin: *p,
            width,
            height,
        };
        self.draw_geo(aabb.to_rect(), color);
    }

    pub fn draw_origin(&mut self, p: &P2, color: Color) {}

    pub fn draw_selector(&mut self, aabb: &mut Aabb, corner_length: f64) {
        self.draw_selector_part(&aabb.points(), [0., 0., 0., 1.], corner_length * 1.2);
        self.draw_selector_part(&aabb.points(), [0., 1., 0., 1.], corner_length);
    }

    fn draw_selector_part(&mut self, points: &RectPoints, color: Color, corner_length: f64) {
        // top right
        self.draw_corner(points[0], true, true, color, corner_length);
        // bottom right
        self.draw_corner(points[1], false, true, color, corner_length);
        // bottom left
        self.draw_corner(points[2], false, false, color, corner_length);
        // top left
        self.draw_corner(points[3], true, false, color, corner_length);
    }

    fn draw_corner(&mut self, point: P2, top: bool, right: bool, color: Color, corner_length: f64) {
        let corner_length = corner_length.abs();
        let x_diff = if right { -corner_length } else { corner_length };
        let y_diff = if top { -corner_length } else { corner_length };
        let v = [
            point,
            P2::new(point.x - x_diff * 0.2, point.y - y_diff * 0.2),
            P2::new(point.x, point.y + y_diff),
            P2::new(point.x + x_diff, point.y),
            P2::new(point.x - x_diff * 0.2, point.y + y_diff),
            P2::new(point.x + x_diff, point.y - y_diff * 0.2),
        ];
        self.triangles.append(&mut vec![
            (v[0], color),
            (v[1], color),
            (v[2], color),
            (v[0], color),
            (v[1], color),
            (v[3], color),
            (v[1], color),
            (v[2], color),
            (v[4], color),
            (v[1], color),
            (v[3], color),
            (v[5], color),
        ]);
    }

    pub fn get_lines(&mut self) -> Vec<(P2, Color)> {
        let lines = std::mem::take(&mut self.lines);
        lines
    }

    pub fn get_triangles(&mut self) -> Vec<(P2, Color)> {
        let triangles = std::mem::take(&mut self.triangles);
        triangles
    }

    pub fn resize(&mut self, new_canvas_bounds: &Rect) {
        self.canvas_bounds = *new_canvas_bounds;
    }
}
