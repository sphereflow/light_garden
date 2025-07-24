use collision2d::geo::*;
use serde::{Deserialize, Serialize};

use super::Color;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObjectE {
    StraightMirror(StraightMirror),
    CurvedMirror(CurvedMirror),
    Circle(Circle),
    Rect(Rect),
    Lens(Lens),
    ConvexPolygon(ConvexPolygon),
    Ellipse(Ellipse),
    Geo(Geo),
}

impl ObjectE {
    pub fn new_mirror(a: P2, b: P2) -> Self {
        ObjectE::StraightMirror(StraightMirror::new(LineSegment::from_ab(a, b)))
    }
    pub fn new_curved_mirror(cubic: &CubicBezier) -> Self {
        ObjectE::CurvedMirror(CurvedMirror::new(*cubic))
    }
    pub fn new_circle(origin: P2, radius: Float) -> Self {
        ObjectE::Circle(Circle { origin, radius })
    }
    pub fn new_rect(origin: P2, width: Float, height: Float) -> Self {
        ObjectE::Rect(Rect::new(origin, Rot2::identity(), width, height))
    }
    pub fn new_lens(origin: P2, radius: Float, distance: Float) -> Self {
        ObjectE::Lens(Lens::new(origin, radius, distance))
    }
    pub fn new_convex_polygon(points: &[P2]) -> Self {
        ObjectE::ConvexPolygon(ConvexPolygon::new_convex_hull(points))
    }
    pub fn new_ellipse(origin: P2, a: Float, b: Float) -> Self {
        ObjectE::Ellipse(Ellipse {
            origin,
            a,
            b,
            rot: Rotation2::new(0.0),
        })
    }
    pub fn new_geo(geo: Geo) -> Self {
        ObjectE::Geo(geo)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Object {
    pub object_enum: ObjectE,
    pub material_opt: Option<Material>,
    pub moved: bool,
}

impl Object {
    pub fn new_mirror(a: P2, b: P2) -> Self {
        Object {
            object_enum: ObjectE::new_mirror(a, b),
            material_opt: None,
            moved: true,
        }
    }
    pub fn new_curved_mirror(cubic: &CubicBezier) -> Self {
        Object {
            object_enum: ObjectE::new_curved_mirror(cubic),
            material_opt: None,
            moved: true,
        }
    }
    pub fn new_circle(origin: P2, radius: Float) -> Self {
        Object {
            object_enum: ObjectE::new_circle(origin, radius),
            material_opt: Some(Material::default()),
            moved: true,
        }
    }
    pub fn new_rect(origin: P2, width: Float, height: Float) -> Self {
        Object {
            object_enum: ObjectE::new_rect(origin, width, height),
            material_opt: Some(Material::default()),
            moved: true,
        }
    }
    pub fn new_lens(origin: P2, radius: Float, distance: Float) -> Self {
        Object {
            object_enum: ObjectE::new_lens(origin, radius, distance),
            material_opt: Some(Material::default()),
            moved: true,
        }
    }
    pub fn new_convex_polygon(points: &[P2]) -> Self {
        Object {
            object_enum: ObjectE::new_convex_polygon(points),
            material_opt: Some(Material::default()),
            moved: true,
        }
    }
    pub fn new_ellipse(origin: P2, a: Float, b: Float) -> Self {
        Object {
            object_enum: ObjectE::new_ellipse(origin, a, b),
            material_opt: Some(Material::default()),
            moved: true,
        }
    }
    pub fn new_geo(geo: Geo) -> Self {
        Object {
            object_enum: ObjectE::Geo(geo),
            material_opt: Some(Material::default()),
            moved: true,
        }
    }
    pub fn get_material(&self) -> Option<Material> {
        self.material_opt
    }
    pub fn material_mut(&mut self) -> Option<&mut Material> {
        self.material_opt.as_mut()
    }
}

impl HasOrigin for ObjectE {
    fn get_origin(&self) -> P2 {
        match self {
            ObjectE::StraightMirror(m) => m.line_segment.get_origin(),
            ObjectE::CurvedMirror(cm) => cm.cubic.get_origin(),
            ObjectE::Circle(c) => c.get_origin(),
            ObjectE::Rect(r) => r.get_origin(),
            ObjectE::Lens(l) => l.l.get_origin(),
            ObjectE::ConvexPolygon(cp) => cp.get_origin(),
            ObjectE::Ellipse(e) => e.get_origin(),
            ObjectE::Geo(g) => g.get_origin(),
        }
    }
    fn set_origin(&mut self, origin: P2) {
        match self {
            ObjectE::StraightMirror(m) => m.line_segment.set_origin(origin),
            ObjectE::CurvedMirror(cm) => cm.cubic.set_origin(origin),
            ObjectE::Circle(c) => c.set_origin(origin),
            ObjectE::Rect(r) => r.set_origin(origin),
            ObjectE::Lens(l) => l.l.set_origin(origin),
            ObjectE::ConvexPolygon(cp) => cp.set_origin(origin),
            ObjectE::Ellipse(e) => e.set_origin(origin),
            ObjectE::Geo(g) => g.set_origin(origin),
        }
    }
}

impl HasOrigin for Object {
    fn get_origin(&self) -> P2 {
        self.object_enum.get_origin()
    }
    fn set_origin(&mut self, origin: P2) {
        self.moved = true;
        self.object_enum.set_origin(origin);
    }
}

impl Rotate for ObjectE {
    fn get_rotation(&self) -> Rot2 {
        match self {
            ObjectE::StraightMirror(m) => m.line_segment.get_rotation(),
            ObjectE::CurvedMirror(cm) => cm.cubic.get_rotation(),
            ObjectE::Circle(c) => c.get_rotation(),
            ObjectE::Rect(r) => r.get_rotation(),
            ObjectE::Lens(l) => l.l.get_rotation(),
            ObjectE::ConvexPolygon(cp) => cp.get_rotation(),
            ObjectE::Ellipse(e) => e.get_rotation(),
            ObjectE::Geo(g) => g.get_rotation(),
        }
    }
    fn set_rotation(&mut self, rotation: &Rot2) {
        match self {
            ObjectE::StraightMirror(m) => m.line_segment.set_rotation(rotation),
            ObjectE::CurvedMirror(cm) => cm.cubic.set_rotation(rotation),
            ObjectE::Circle(c) => c.set_rotation(rotation),
            ObjectE::Rect(r) => r.set_rotation(rotation),
            ObjectE::Lens(l) => l.l.set_rotation(rotation),
            ObjectE::ConvexPolygon(cp) => cp.set_rotation(rotation),
            ObjectE::Ellipse(e) => e.set_rotation(rotation),
            ObjectE::Geo(g) => g.set_rotation(rotation),
        }
    }
}

impl Rotate for Object {
    fn get_rotation(&self) -> Rot2 {
        self.object_enum.get_rotation()
    }
    fn set_rotation(&mut self, rotation: &Rot2) {
        self.moved = true;
        self.object_enum.set_rotation(rotation);
    }
}

impl Mirror for ObjectE {
    fn mirror_x(&self) -> Self {
        match self {
            ObjectE::StraightMirror(m) => ObjectE::StraightMirror(m.mirror_x()),
            ObjectE::CurvedMirror(cm) => ObjectE::CurvedMirror(cm.mirror_x()),
            ObjectE::Circle(c) => ObjectE::Circle(c.mirror_x()),
            ObjectE::Rect(r) => ObjectE::Rect(r.mirror_x()),
            ObjectE::Lens(l) => ObjectE::Lens(l.mirror_x()),
            ObjectE::ConvexPolygon(cp) => ObjectE::ConvexPolygon(cp.mirror_x()),
            ObjectE::Ellipse(e) => ObjectE::Ellipse(e.mirror_x()),
            ObjectE::Geo(g) => ObjectE::Geo(g.mirror_x()),
        }
    }
    fn mirror_y(&self) -> Self {
        match self {
            ObjectE::StraightMirror(m) => ObjectE::StraightMirror(m.mirror_y()),
            ObjectE::CurvedMirror(cm) => ObjectE::CurvedMirror(cm.mirror_y()),
            ObjectE::Circle(c) => ObjectE::Circle(c.mirror_y()),
            ObjectE::Rect(r) => ObjectE::Rect(r.mirror_y()),
            ObjectE::Lens(l) => ObjectE::Lens(l.mirror_y()),
            ObjectE::ConvexPolygon(cp) => ObjectE::ConvexPolygon(cp.mirror_y()),
            ObjectE::Ellipse(e) => ObjectE::Ellipse(e.mirror_y()),
            ObjectE::Geo(g) => ObjectE::Geo(g.mirror_y()),
        }
    }
}

impl Mirror for Object {
    fn mirror_x(&self) -> Self {
        Object {
            object_enum: self.object_enum.mirror_x(),
            material_opt: self.material_opt,
            moved: true,
        }
    }
    fn mirror_y(&self) -> Self {
        Object {
            object_enum: self.object_enum.mirror_y(),
            material_opt: self.material_opt,
            moved: true,
        }
    }
}

impl Contains for ObjectE {
    fn contains(&self, p: &P2) -> bool {
        match self {
            ObjectE::StraightMirror(_) => false,
            ObjectE::CurvedMirror(_) => false,
            ObjectE::Circle(c) => c.contains(p),
            ObjectE::Rect(r) => r.contains(p),
            ObjectE::Lens(l) => l.get_logic().contains(p),
            ObjectE::ConvexPolygon(cp) => cp.contains(p),
            ObjectE::Ellipse(e) => e.contains(p),
            ObjectE::Geo(g) => g.contains(p),
        }
    }
}

impl Contains for Object {
    fn contains(&self, p: &P2) -> bool {
        self.object_enum.contains(p)
    }
}

impl Distance for ObjectE {
    fn distance(&self, p: &P2) -> Float {
        match self {
            ObjectE::StraightMirror(m) => m.line_segment.distance(p),
            ObjectE::CurvedMirror(cm) => distance(&cm.cubic.get_origin(), p),
            ObjectE::Circle(c) => c.distance(p),
            ObjectE::Rect(r) => r.distance(p),
            ObjectE::Lens(l) => l.get_logic().distance(p),
            ObjectE::ConvexPolygon(cp) => cp.distance(p),
            ObjectE::Ellipse(e) => e.distance(p),
            ObjectE::Geo(g) => g.distance(p),
        }
    }
}

impl Distance for Object {
    fn distance(&self, p: &P2) -> Float {
        self.object_enum.distance(p)
    }
}

impl HasGeometry for ObjectE {
    fn get_geometry(&self) -> Geo {
        match self {
            ObjectE::StraightMirror(mirror) => mirror.get_geometry(),
            ObjectE::CurvedMirror(cm) => cm.get_geometry(),
            ObjectE::Circle(c) => Geo::GeoCircle(*c),
            ObjectE::Rect(r) => Geo::GeoRect(*r),
            ObjectE::Lens(l) => Geo::GeoLogic(l.get_logic()),
            ObjectE::ConvexPolygon(cp) => Geo::GeoConvexPolygon(cp.clone()),
            ObjectE::Ellipse(e) => Geo::GeoEllipse(*e),
            ObjectE::Geo(g) => g.clone(),
        }
    }
}

impl HasGeometry for Object {
    fn get_geometry(&self) -> Geo {
        self.object_enum.get_geometry()
    }
}

impl HasAabb for ObjectE {
    fn get_aabb(&self) -> Aabb {
        self.get_geometry().get_aabb()
    }
}

impl HasAabb for Object {
    fn get_aabb(&self) -> Aabb {
        self.object_enum.get_aabb()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StraightMirror {
    pub line_segment: LineSegment,
}

impl StraightMirror {
    pub fn new(line_segment: LineSegment) -> Self {
        StraightMirror { line_segment }
    }
}

impl Mirror for StraightMirror {
    fn mirror_x(&self) -> Self {
        StraightMirror {
            line_segment: self.line_segment.mirror_x(),
        }
    }
    fn mirror_y(&self) -> Self {
        StraightMirror {
            line_segment: self.line_segment.mirror_y(),
        }
    }
}

impl HasGeometry for StraightMirror {
    fn get_geometry(&self) -> Geo {
        Geo::GeoLineSegment(self.line_segment)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CurvedMirror {
    pub cubic: CubicBezier,
}

impl CurvedMirror {
    pub fn new(cubic: CubicBezier) -> Self {
        CurvedMirror { cubic }
    }

    pub fn get_control_lines(&self) -> Vec<(P2, Color)> {
        let mut lines = Vec::new();
        let red = [1., 0., 0., 1.];
        lines.push((self.cubic.points[0], red));
        lines.push((self.cubic.points[1], red));
        lines.push((self.cubic.points[1], red));
        lines.push((self.cubic.points[2], red));
        lines.push((self.cubic.points[2], red));
        lines.push((self.cubic.points[3], red));
        lines
    }
}

impl Mirror for CurvedMirror {
    fn mirror_x(&self) -> Self {
        CurvedMirror {
            cubic: self.cubic.mirror_x(),
        }
    }
    fn mirror_y(&self) -> Self {
        CurvedMirror {
            cubic: self.cubic.mirror_y(),
        }
    }
}

impl HasGeometry for CurvedMirror {
    fn get_geometry(&self) -> Geo {
        Geo::GeoCubicBezier(self.cubic)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

impl Mirror for Lens {
    fn mirror_x(&self) -> Self {
        Lens {
            l: self.l.mirror_x(),
        }
    }
    fn mirror_y(&self) -> Self {
        Lens {
            l: self.l.mirror_y(),
        }
    }
}

impl HasGeometry for Lens {
    fn get_geometry(&self) -> Geo {
        Geo::GeoLogic(self.l.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub refractive_index: Float,
}

impl Default for Material {
    fn default() -> Self {
        Material {
            refractive_index: 1.2,
        }
    }
}
