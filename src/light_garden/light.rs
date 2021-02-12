extern crate nalgebra as na;

use collision2d::geo::*;
use na::{Point2, Vector2};

pub type Color = [f32; 4];

#[derive(PartialEq, Debug, Clone)]
pub enum Light {
    PointLight(PointLight),
    DirectionalLight(DirectionalLight),
    SpotLight(SpotLight),
}

impl Light {
    pub fn get_rays(&self) -> &Vec<Ray> {
        match self {
            Light::PointLight(l) => &l.rays,
            Light::DirectionalLight(l) => &l.rays,
            Light::SpotLight(l) => &l.rays,
        }
    }
    pub fn color_mut(&mut self) -> &mut Color {
        match self {
            Light::PointLight(l) => &mut l.color,
            Light::DirectionalLight(l) => &mut l.color,
            Light::SpotLight(l) => &mut l.color,
        }
    }
    pub fn get_color(&self) -> Color {
        match self {
            Light::PointLight(l) => l.color,
            Light::DirectionalLight(l) => l.color,
            Light::SpotLight(l) => l.color,
        }
    }
    pub fn set_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        match self {
            Light::PointLight(l) => l.set_color([r, g, b, a]),
            Light::DirectionalLight(l) => l.set_color([r, g, b, a]),
            Light::SpotLight(l) => l.set_color([r, g, b, a]),
        }
    }
    pub fn set_num_rays(&mut self, num_rays: u32) {
        match self {
            Light::PointLight(l) => l.set_num_rays(num_rays),
            Light::DirectionalLight(l) => l.set_num_rays(num_rays),
            Light::SpotLight(_l) => {}
        }
    }
    pub fn get_num_rays(&self) -> u32 {
        self.get_rays().len() as u32
    }
}

impl HasOrigin for Light {
    fn get_origin(&self) -> P2 {
        match self {
            Light::PointLight(l) => l.get_origin(),
            Light::DirectionalLight(l) => l.get_origin(),
            Light::SpotLight(l) => l.get_origin(),
        }
    }
    fn set_origin(&mut self, origin: P2) {
        match self {
            Light::PointLight(l) => l.set_origin(origin),
            Light::DirectionalLight(l) => l.set_origin(origin),
            Light::SpotLight(l) => l.set_origin(origin),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct DirectionalLight {
    position: P2,
    color: Color,
    rays: Vec<Ray>,
    ray_distance: Float,
    direction: V2,
}

impl DirectionalLight {
    pub fn new(ray_distance: Float, num_rays: u32, color: Color, _screen_bounds: Rect) -> Self {
        let mut ret = DirectionalLight {
            position: Point2::new(0., 0.),
            color,
            rays: Vec::new(),
            ray_distance,
            direction: Vector2::new(0., -1.),
        };
        ret.set_num_rays(num_rays);
        ret
    }

    fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn set_num_rays(&mut self, _num_rays: u32) {}
}

impl HasOrigin for DirectionalLight {
    fn get_origin(&self) -> P2 {
        self.position
    }
    fn set_origin(&mut self, origin: P2) {
        self.position = origin;
        unimplemented!();
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct PointLight {
    position: P2,
    pub color: Color,
    rays: Vec<Ray>,
}

impl PointLight {
    pub fn new(position: P2, num_rays: u32, color: Color) -> Self {
        let mut light = PointLight {
            position,
            color,
            rays: Vec::new(),
        };
        light.set_num_rays(num_rays);
        light
    }

    fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn set_num_rays(&mut self, num_rays: u32) {
        self.rays = Vec::with_capacity(num_rays as usize);
        for i in 0..num_rays {
            let f = i as Float * std::f64::consts::PI * 2. / num_rays as Float;
            let (sine, cosine) = f.sin_cos();
            self.rays
                .push(Ray::from_origin(self.position, Vector2::new(cosine, sine)));
        }
    }
}

impl HasOrigin for PointLight {
    fn get_origin(&self) -> P2 {
        self.position
    }
    fn set_origin(&mut self, origin: P2) {
        self.position = origin;
        for ray in self.rays.iter_mut() {
            ray.set_origin(self.position);
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct SpotLight {
    position: P2,
    color: Color,
    rays: Vec<Ray>,
    pub spot_angle: Float,
    spot_direction: V2,
}

impl SpotLight {
    pub fn new(
        position: P2,
        spot_angle: Float,
        spot_direction: V2,
        num_rays: u32,
        color: Color,
    ) -> Self {
        let mut light = SpotLight {
            position,
            color,
            rays: Vec::new(),
            spot_angle,
            spot_direction,
        };
        light.set_num_rays(num_rays);
        light
    }

    fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn set_num_rays(&mut self, num_rays: u32) {}
}

impl HasOrigin for SpotLight {
    fn get_origin(&self) -> P2 {
        self.position
    }

    fn set_origin(&mut self, origin: P2) {
        self.position = origin;
        for ray in self.rays.iter_mut() {
            ray.set_origin(self.position);
        }
    }
}
