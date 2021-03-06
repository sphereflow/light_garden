use collision2d::geo::traits::*;
use collision2d::geo::*;

#[derive(Debug, Clone)]
pub enum Object {
    Mirror(Mirror),
    CurvedMirror(CurvedMirror),
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
            Rect::new(origin, Rot2::identity(), width, height),
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
            Object::CurvedMirror(_) => Material::default(),
            Object::Circle(_, m) => *m,
            Object::Rect(_, m) => *m,
            Object::Lens(_, m) => *m,
            Object::Geo(_, m) => *m,
        }
    }
    pub fn material_mut(&mut self) -> Option<&mut Material> {
        match self {
            Object::Mirror(_) => None,
            Object::CurvedMirror(_) => None,
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
            Object::CurvedMirror(cm) => cm.cubic.get_origin(),
            Object::Circle(c, _) => c.get_origin(),
            Object::Rect(r, _) => r.get_origin(),
            Object::Lens(l, _) => l.l.get_origin(),
            Object::Geo(g, _) => g.get_origin(),
        }
    }
    fn set_origin(&mut self, origin: P2) {
        match self {
            Object::Mirror(m) => m.line_segment.set_origin(origin),
            Object::CurvedMirror(cm) => cm.cubic.set_origin(origin),
            Object::Circle(c, _) => c.set_origin(origin),
            Object::Rect(r, _) => r.set_origin(origin),
            Object::Lens(l, _) => l.l.set_origin(origin),
            Object::Geo(g, _) => g.set_origin(origin),
        }
    }
}

impl Rotate for Object {
    fn get_rotation(&self) -> Rot2 {
        match self {
            Object::Mirror(m) => m.line_segment.get_rotation(),
            Object::CurvedMirror(cm) => cm.cubic.get_rotation(),
            Object::Circle(c, _) => c.get_rotation(),
            Object::Rect(r, _) => r.get_rotation(),
            Object::Lens(l, _) => l.l.get_rotation(),
            Object::Geo(g, _) => g.get_rotation(),
        }
    }
    fn set_rotation(&mut self, rotation: &Rot2) {
        match self {
            Object::Mirror(m) => m.line_segment.set_rotation(rotation),
            Object::CurvedMirror(cm) => cm.cubic.set_rotation(rotation),
            Object::Circle(c, _) => c.set_rotation(rotation),
            Object::Rect(r, _) => r.set_rotation(rotation),
            Object::Lens(l, _) => l.l.set_rotation(rotation),
            Object::Geo(g, _) => g.set_rotation(rotation),
        }
    }
}

impl Contains for Object {
    fn contains(&self, p: &P2) -> bool {
        match self {
            Object::Mirror(_) => false,
            Object::CurvedMirror(_) => false,
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
            Object::CurvedMirror(cm) => distance(&cm.cubic.get_origin(), p),
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
            Object::CurvedMirror(cm) => cm.get_geometry(),
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

#[derive(Debug, Clone, Copy)]
pub struct CurvedMirror {
    pub cubic: CubicBezier,
}

impl CurvedMirror {
    pub fn new(cubic: CubicBezier) -> Self {
        CurvedMirror { cubic }
    }
}

impl HasGeometry for CurvedMirror {
    fn get_geometry(&self) -> Geo {
        Geo::GeoCubicBezier(self.cubic)
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
                Rot2::identity(),
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
