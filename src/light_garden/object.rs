use collision2d::geo::traits::*;
use collision2d::geo::*;

#[derive(Debug, Clone)]
pub enum Object {
    Mirror(Mirror),
    Circle(Circle, Material),
    Rect(Rect, Material),
    Lens(Lens, Material),
    Geo(Geo, Material),
}

impl Object {
    pub fn new_mirror(a: P2, b: P2) -> Self {
        Object::Mirror(Mirror::new(LineSegment::from_ab(a, b)))
    }
    pub fn new_circle(origin: P2, radius: Float, refractive_index: Float) -> Self {
        Object::Circle(Circle { origin, radius }, Material { refractive_index })
    }
    pub fn new_rect(origin: P2, width: Float, height: Float, refractive_index: Float) -> Self {
        Object::Rect(
            Rect::new(origin, V2::from([1., 0.]), width, height),
            Material { refractive_index },
        )
    }
    pub fn new_lens(origin: P2, radius: Float, distance: Float, refractive_index: Float) -> Self {
        Object::Lens(
            Lens::new(origin, radius, distance),
            Material { refractive_index },
        )
    }
    pub fn new_geo(geo: Geo, refractive_index: Float) -> Self {
        Object::Geo(geo, Material { refractive_index })
    }
    pub fn get_material(&self) -> Material {
        match self {
            Object::Mirror(_) => Material::default(),
            Object::Circle(_, m) => *m,
            Object::Rect(_, m) => *m,
            Object::Lens(_, m) => *m,
            Object::Geo(_, m) => *m,
        }
    }
    pub fn material_mut(&mut self) -> Option<&mut Material> {
        match self {
            Object::Mirror(_) => None,
            Object::Circle(_, m) => Some(m),
            Object::Rect(_, m) => Some(m),
            Object::Lens(_, m) => Some(m),
            Object::Geo(_, m) => Some(m),
        }
    }
}

impl HasOrigin for Object {
    fn get_origin(&self) -> P2 {
        match self {
            Object::Mirror(m) => m.line_segment.get_origin(),
            Object::Circle(c, _) => c.get_origin(),
            Object::Rect(r, _) => r.get_origin(),
            Object::Lens(l, _) => l.l.get_origin(),
            Object::Geo(g, _) => g.get_origin(),
        }
    }
    fn set_origin(&mut self, origin: P2) {
        match self {
            Object::Mirror(m) => m.line_segment.set_origin(origin),
            Object::Circle(c, _) => c.set_origin(origin),
            Object::Rect(r, _) => r.set_origin(origin),
            Object::Lens(l, _) => l.l.set_origin(origin),
            Object::Geo(g, _) => g.set_origin(origin),
        }
    }
}

impl Rotate for Object {
    fn get_rotation(&self) -> V2 {
        match self {
            Object::Mirror(m) => m.line_segment.get_rotation(),
            Object::Circle(c, _) => c.get_rotation(),
            Object::Rect(r, _) => r.get_rotation(),
            Object::Lens(l, _) => l.l.get_rotation(),
            Object::Geo(g, _) => g.get_rotation(),
        }
    }
    fn set_rotation(&mut self, x_axis: &V2) {
        match self {
            Object::Mirror(m) => m.line_segment.set_rotation(x_axis),
            Object::Circle(c, _) => c.set_rotation(x_axis),
            Object::Rect(r, _) => r.set_rotation(x_axis),
            Object::Lens(l, _) => l.l.set_rotation(x_axis),
            Object::Geo(g, _) => g.set_rotation(x_axis),
        }
    }
}

impl Contains for Object {
    fn contains(&self, p: &P2) -> bool {
        match self {
            Object::Mirror(_) => false,
            Object::Circle(c, _) => c.contains(p),
            Object::Rect(r, _) => r.contains(p),
            Object::Lens(l, _) => l.get_logic().contains(p),
            Object::Geo(g, _) => g.contains(p),
        }
    }
}

impl Distance for Object {
    fn distance(&self, p: &P2) -> Float {
        match self {
            Object::Mirror(m) => m.line_segment.distance(p),
            Object::Circle(c, _) => c.distance(p),
            Object::Rect(r, _) => r.distance(p),
            Object::Lens(l, _) => l.get_logic().distance(p),
            Object::Geo(g, _) => g.distance(p),
        }
    }
}

impl HasGeometry for Object {
    fn get_geometry(&self) -> Geo {
        match self {
            Object::Mirror(mirror) => mirror.get_geometry(),
            Object::Circle(c, _material) => Geo::GeoCircle(*c),
            Object::Rect(r, _material) => Geo::GeoRect(*r),
            Object::Lens(l, _) => Geo::GeoLogic(l.get_logic()),
            Object::Geo(g, _) => g.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Mirror {
    pub line_segment: LineSegment,
}

impl Mirror {
    pub fn new(line_segment: LineSegment) -> Self {
        Mirror { line_segment }
    }
}

impl HasGeometry for Mirror {
    fn get_geometry(&self) -> Geo {
        Geo::GeoLineSegment(self.line_segment)
    }
}

#[derive(Debug, Clone)]
pub struct Lens {
    pub l: Logic,
}

impl Lens {
    pub fn new(origin: P2, radius: Float, distance: Float) -> Self {
        Lens {
            l: Logic::new(
                LogicOp::And,
                Circle {
                    radius,
                    origin: P2::new(distance * 0.5, 0.),
                }
                .get_geometry(),
                Circle {
                    radius,
                    origin: P2::new(-distance * 0.5, 0.),
                }
                .get_geometry(),
                origin,
                V2::new(1., 0.),
            ),
        }
    }
    pub fn get_logic(&self) -> Logic {
        self.l.clone()
    }
}

impl HasGeometry for Lens {
    fn get_geometry(&self) -> Geo {
        Geo::GeoLogic(self.l.clone())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Material {
    pub refractive_index: Float,
}

impl Default for Material {
    fn default() -> Self {
        Material {
            refractive_index: 1.,
        }
    }
}
