extern crate nalgebra as na;

use collision2d::geo::*;
use na::Vector2;
use serde::{Deserialize, Serialize};

pub type Color = [f32; 4];

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
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
    pub fn set_color(&mut self, red: f32, green: f32, blue: f32, alpha: f32) {
        match self {
            Light::PointLight(light) => light.set_color([red, green, blue, alpha]),
            Light::DirectionalLight(light) => light.set_color([red, green, blue, alpha]),
            Light::SpotLight(light) => light.set_color([red, green, blue, alpha]),
        }
    }
    pub fn set_num_rays(&mut self, num_rays: Option<usize>) {
        match self {
            Light::PointLight(l) => l.set_num_rays(num_rays),
            Light::DirectionalLight(l) => l.set_num_rays(num_rays),
            Light::SpotLight(l) => l.set_num_rays(num_rays),
        }
    }
    pub fn get_num_rays(&self) -> usize {
        match self {
            Light::PointLight(l) => l.num_rays,
            Light::DirectionalLight(l) => l.num_rays,
            Light::SpotLight(l) => l.num_rays,
        }
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

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct DirectionalLight {
    color: Color,
    #[serde(skip)]
    rays: Vec<Ray>,
    num_rays: usize,
    start: LineSegment,
}

impl DirectionalLight {
    pub fn new(color: Color, num_rays: usize, start: LineSegment) -> Self {
        let mut ret = DirectionalLight {
            color,
            rays: Vec::new(),
            num_rays,
            start,
        };
        ret.set_num_rays(None);
        ret
    }

    fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn set_num_rays(&mut self, num_rays: Option<usize>) {
        if let Some(n) = num_rays {
            self.num_rays = n;
        }
        self.rays = Vec::with_capacity(self.num_rays);
        let n = self.start.get_normal();
        for i in 0..self.num_rays {
            self.rays.push(Ray::from_origin(
                self.start.eval_at_r(-(i as f64) / self.num_rays as f64),
                n.into_inner(),
            ));
        }
    }
}

impl HasOrigin for DirectionalLight {
    fn get_origin(&self) -> P2 {
        self.start.get_origin()
    }
    fn set_origin(&mut self, origin: P2) {
        self.start.set_origin(origin);
        self.set_num_rays(None);
    }
}

impl Rotate for DirectionalLight {
    fn get_rotation(&self) -> Rot2 {
        self.start.get_rotation()
    }
    fn set_rotation(&mut self, rotation: &Rot2) {
        self.start.set_rotation(rotation);
        self.set_num_rays(None);
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct PointLight {
    position: P2,
    pub color: Color,
    #[serde(skip)]
    rays: Vec<Ray>,
    num_rays: usize,
}

impl PointLight {
    pub fn new(position: P2, num_rays: usize, color: Color) -> Self {
        let mut light = PointLight {
            position,
            color,
            rays: Vec::new(),
            num_rays,
        };
        light.set_num_rays(None);
        light
    }

    fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn set_num_rays(&mut self, num_rays: Option<usize>) {
        if let Some(n) = num_rays {
            self.num_rays = n;
        }
        self.rays = Vec::with_capacity(self.num_rays);
        for i in 0..self.num_rays {
            let f = i as Float * std::f64::consts::PI * 2. / self.num_rays as Float;
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

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct SpotLight {
    position: P2,
    color: Color,
    #[serde(skip)]
    rays: Vec<Ray>,
    num_rays: usize,
    pub spot_angle: Float,
    spot_direction: V2,
}

impl SpotLight {
    pub fn new(
        position: P2,
        spot_angle: Float,
        spot_direction: V2,
        num_rays: usize,
        color: Color,
    ) -> Self {
        let mut light = SpotLight {
            position,
            color,
            rays: Vec::new(),
            num_rays,
            spot_angle,
            spot_direction,
        };
        light.set_num_rays(None);
        light
    }

    fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    // moving counterclockwise results in a more positive angle
    pub fn set_num_rays(&mut self, num_rays: Option<usize>) {
        if let Some(n) = num_rays {
            self.num_rays = n;
        }
        self.rays = Vec::with_capacity(self.num_rays);
        let direction_angle = if self.spot_direction.x.abs() < EPSILON {
            if self.spot_direction.y >= 0. {
                std::f64::consts::PI * 0.5
            } else {
                -std::f64::consts::PI * 0.5
            }
        } else {
            (self.spot_direction.y / self.spot_direction.x).atan()
        };
        let min_angle = direction_angle - 0.5 * self.spot_angle;
        for step in 1..=self.num_rays {
            let angle = min_angle + (step as f64 / self.num_rays as f64) * self.spot_angle;
            let (ydir, xdir) = angle.sin_cos();
            let sign = self.spot_direction.x.signum();
            self.rays.push(Ray::from_origin(
                self.position,
                V2::new(sign * xdir, sign * ydir),
            ));
        }
    }
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

impl Rotate for SpotLight {
    fn get_rotation(&self) -> Rot2 {
        Rot2::rotation_between(&V2::new(1., 0.), &self.spot_direction)
    }
    fn set_rotation(&mut self, rotation: &Rot2) {
        self.spot_direction = rotation.matrix().column(0).clone_owned();
        self.set_num_rays(None);
    }
}
