extern crate nalgebra as na;

use collision2d::geo::*;
use instant::Instant;
pub use light::*;
use na::{distance, Point2};
pub use object::*;
use rayon::prelude::*;
use std::{collections::VecDeque, f64::consts::FRAC_PI_4};

pub mod light;
pub mod object;

pub struct LightGarden {
    mouse_pos: Point2<Float>,
    pub lights: Vec<Light>,
    pub max_bounce: u32,
    pub num_rays: u32,
    pub objects: Vec<Object>,
    drawing_object: Option<Object>,
    drawing_light: Option<Light>,
    pub selected_object: Option<usize>,
    pub selected_light: Option<usize>,
    pub selected_color: Color,
    pub color_state_descriptor: wgpu::ColorTargetState,
    pub recreate_pipeline: bool,
    pub canvas_bounds: Rect,
    pub ray_width: f64,
    pub mode: Mode,
    pub refractive_index: Float,
    pub chunk_size: usize,
    pub cutoff_color: Color,
    render_to_texture: bool,
    trace_time_vd: VecDeque<f64>,
}

impl LightGarden {
    pub fn new(canvas_bounds: Rect, descriptor_format: wgpu::TextureFormat) -> LightGarden {
        let light = Light::PointLight(PointLight::new(Point2::new(-0.1, 0.1), 10000, [0.01; 4]));
        let lens = Object::new_lens(P2::new(0.7, 0.), 2., 3.8, 5.);
        let mut cubic1 = CubicBezier::new_sample();
        cubic1.scale(0.5, 0.5);
        let curved_mirror1 = Object::CurvedMirror(CurvedMirror { cubic: cubic1 });
        let mut cubic2 = CubicBezier::new_sample2();
        cubic2.scale(0.5, 0.5);
        let curved_mirror2 = Object::CurvedMirror(CurvedMirror { cubic: cubic2 });
        let color_state_descriptor = wgpu::ColorTargetState {
            format: descriptor_format,
            alpha_blend: wgpu::BlendState {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            color_blend: wgpu::BlendState {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            write_mask: wgpu::ColorWrite::ALL,
        };
        LightGarden {
            mouse_pos: Point2::new(0., 0.),
            lights: vec![light],
            objects: vec![lens, curved_mirror1, curved_mirror2],
            drawing_object: None,
            drawing_light: None,
            selected_object: None,
            selected_light: None,
            color_state_descriptor,
            recreate_pipeline: true,
            max_bounce: 5,
            num_rays: 2000,
            canvas_bounds,
            selected_color: [0.02, 0.03, 0.05, 0.01],
            cutoff_color: [0.001; 4],
            render_to_texture: true,
            ray_width: 1.0,
            mode: Mode::NoMode,
            refractive_index: 2.,
            chunk_size: 100,
            trace_time_vd: VecDeque::new(),
        }
    }

    pub fn update_mouse_position(&mut self, position: P2) {
        self.mouse_pos = position;
        let aspect = self.canvas_bounds.width / self.canvas_bounds.height;
        self.mouse_pos.x *= aspect;
        if !self.canvas_bounds.contains(&self.mouse_pos) {
            return;
        }
        self.update();
    }

    pub fn update(&mut self) {
        match self.mode {
            Mode::DrawMirrorEnd { start } => {
                self.drawing_object = Some(Object::new_mirror(start, self.mouse_pos));
            }

            Mode::DrawCircleEnd { start } => {
                self.drawing_object = Some(Object::new_circle(
                    start,
                    distance(&start, &self.mouse_pos),
                    self.refractive_index,
                ));
            }

            Mode::DrawRectEnd { start } => {
                let vdiff_t2 = 2. * (self.mouse_pos - start);
                let width = vdiff_t2[0].abs();
                let height = vdiff_t2[1].abs();
                self.drawing_object = Some(Object::new_rect(
                    start,
                    width,
                    height,
                    self.refractive_index,
                ));
            }

            Mode::DrawPointLight => {
                self.drawing_light = Some(Light::PointLight(PointLight::new(
                    self.mouse_pos,
                    self.num_rays,
                    self.selected_color,
                )));
            }

            Mode::DrawSpotLightStart => {
                self.drawing_light = Some(Light::SpotLight(SpotLight::new(
                    self.mouse_pos,
                    FRAC_PI_4,
                    V2::new(1., 0.),
                    self.num_rays,
                    self.selected_color,
                )));
            }

            Mode::DrawSpotLightEnd { origin } => {
                self.drawing_light = Some(Light::SpotLight(SpotLight::new(
                    origin,
                    FRAC_PI_4,
                    self.mouse_pos - origin,
                    self.num_rays,
                    self.selected_color,
                )));
            }

            Mode::DrawDirectionalLightEnd { start } => {
                self.drawing_light = Some(Light::DirectionalLight(DirectionalLight::new(
                    self.selected_color,
                    self.num_rays,
                    LineSegment::from_ab(start, self.mouse_pos),
                )));
            }

            Mode::Move => {
                let mouse_pos = self.mouse_pos;
                if let Some(obj) = self.get_selected_object() {
                    obj.set_origin(mouse_pos);
                }
                if let Some(ix) = self.selected_light {
                    self.lights[ix].set_origin(mouse_pos);
                }
            }

            Mode::Rotate => {
                let mouse_pos = self.mouse_pos;
                if let Some(obj) = self.get_selected_object() {
                    obj.y_axis_look_at(&mouse_pos);
                }
                if let Some(Light::SpotLight(spot)) = self.get_selected_light() {
                    spot.x_axis_look_at(&mouse_pos);
                }
                if let Some(Light::DirectionalLight(directional_light)) = self.get_selected_light() {
                    directional_light.y_axis_look_at(&mouse_pos);
                }
            }

            _ => {}
        }
    }

    pub fn mouse_clicked(&mut self) {
        if !self.canvas_bounds.contains(&self.mouse_pos) {
            return;
        }
        match self.mode {
            Mode::NoMode => {
                self.selected_object = None;
                self.selected_light = None;
            }

            Mode::Selecting(None) => {
                self.selected_object = None;
                self.selected_light = None;
                let mut min_distance = Float::MAX;
                for (ix, o) in self.objects.iter().enumerate() {
                    let dist = o.distance(&self.mouse_pos);
                    if dist < min_distance {
                        min_distance = dist;
                        self.selected_object = Some(ix);
                    }
                }
                for (ix, l) in self.lights.iter().enumerate() {
                    let dist = distance(&l.get_origin(), &self.mouse_pos);
                    if dist < min_distance {
                        min_distance = dist;
                        self.selected_object = None;
                        self.selected_light = Some(ix);
                    }
                }
                self.mode = Mode::Selected;
            }

            Mode::Selecting(Some(op)) => {
                if let Some(current_ix) = self.selected_object {
                    let mut min_distance = Float::MAX;
                    let mut click_selected = None;
                    // find closest object
                    for (ix, o) in self.objects.iter().enumerate() {
                        let dist = o.distance(&self.mouse_pos);
                        if dist < min_distance {
                            min_distance = dist;
                            click_selected = Some(ix);
                        }
                    }
                    if let Some(click_ix) = click_selected {
                        if current_ix == click_ix {
                            // both objects are the same -> abort
                            self.mode = Mode::Selected;
                            return;
                        } else {
                            let geo_a = self.objects[current_ix].get_geometry();
                            let geo_b = self.objects[click_ix].get_geometry();
                            let geo = match op {
                                LogicOp::And => geo_a & geo_b,
                                LogicOp::Or => geo_a | geo_b,
                                LogicOp::AndNot => geo_a.and_not(geo_b),
                            };
                            self.objects[current_ix.min(click_ix)] = Object::new_geo(
                                geo,
                                self.objects[current_ix].get_material().refractive_index,
                            );
                            // current_ix != click_ix
                            self.objects.remove(current_ix.max(click_ix));
                            self.mode = Mode::Selected;
                            self.selected_object = Some(current_ix.min(click_ix));
                        }
                    }
                } else {
                    // there is no currently selected object so performing a logic op is not possible
                    self.mode = Mode::NoMode;
                }
            }

            Mode::Selected => {}

            Mode::Move => {
                self.mode = Mode::Selected;
            }

            Mode::Rotate => {
                self.mode = Mode::Selected;
            }

            Mode::DrawPointLight => {
                self.lights.push(Light::PointLight(PointLight::new(
                    self.mouse_pos,
                    self.num_rays,
                    self.selected_color,
                )));
                self.drawing_light = None;
                self.mode = Mode::NoMode;
            }

            Mode::DrawSpotLightStart => {
                self.mode = Mode::DrawSpotLightEnd {
                    origin: self.mouse_pos,
                };
            }

            Mode::DrawSpotLightEnd { origin } => {
                self.lights.push(Light::SpotLight(SpotLight::new(
                    origin,
                    std::f64::consts::FRAC_PI_4,
                    self.mouse_pos - origin,
                    self.num_rays,
                    self.selected_color,
                )));
                self.drawing_light = None;
                self.mode = Mode::NoMode;
            }

            Mode::DrawDirectionalLightStart => {
                self.mode = Mode::DrawDirectionalLightEnd {
                    start: self.mouse_pos,
                };
            }

            Mode::DrawDirectionalLightEnd { start } => {
                self.lights
                    .push(Light::DirectionalLight(DirectionalLight::new(
                        self.selected_color,
                        self.num_rays,
                        LineSegment::from_ab(start, self.mouse_pos),
                    )));
                self.drawing_light = None;
                self.mode = Mode::NoMode;
            }

            Mode::DrawMirrorStart => {
                self.mode = Mode::DrawMirrorEnd {
                    start: self.mouse_pos,
                };
            }

            Mode::DrawMirrorEnd { start } => {
                self.objects.push(Object::new_mirror(start, self.mouse_pos));
                self.drawing_object = None;
                self.mode = Mode::NoMode;
            }

            Mode::DrawCircleStart => {
                self.mode = Mode::DrawCircleEnd {
                    start: self.mouse_pos,
                };
            }

            Mode::DrawCircleEnd { start } => {
                self.objects.push(Object::new_circle(
                    start,
                    distance(&start, &self.mouse_pos),
                    self.refractive_index,
                ));
                self.drawing_object = None;
                self.mode = Mode::NoMode;
            }

            Mode::DrawRectStart => {
                self.mode = Mode::DrawRectEnd {
                    start: self.mouse_pos,
                };
            }

            Mode::DrawRectEnd { start } => {
                let vdiff_t2 = 2. * (self.mouse_pos - start);
                let width = vdiff_t2[0].abs();
                let height = vdiff_t2[1].abs();
                self.objects.push(Object::new_rect(
                    start,
                    width,
                    height,
                    self.refractive_index,
                ));
                self.drawing_object = None;
                self.mode = Mode::NoMode;
            }
        }
    }

    pub fn clear(&mut self) {
        self.lights = Vec::new();
        self.objects = Vec::new();
        self.drawing_object = None;
        self.selected_object = None;
        self.selected_light = None;
    }

    pub fn clear_objects(&mut self) {
        self.objects = Vec::new();
        self.drawing_object = None;
    }

    pub fn get_selected_object(&mut self) -> Option<&mut Object> {
        if let Some(ix) = self.selected_object {
            Some(&mut self.objects[ix])
        } else {
            None
        }
    }

    pub fn get_selected_light(&mut self) -> Option<&mut Light> {
        if let Some(ix) = self.selected_light {
            Some(&mut self.lights[ix])
        } else {
            None
        }
    }

    pub fn delete_selected(&mut self) {
        if let Some(ix) = self.selected_light {
            self.lights.remove(ix);
        }
        if let Some(ix) = self.selected_object {
            self.objects.remove(ix);
        }
        self.deselect();
    }

    pub fn deselect(&mut self) {
        self.selected_light = None;
        self.selected_object = None;
        self.drawing_object = None;
        self.mode = Mode::NoMode;
    }

    pub fn copy_selected(&mut self) {
        if let Some(ix) = self.selected_object {
            let mut cpy = self.objects[ix].clone();
            let pos = cpy.get_origin();
            cpy.set_origin(pos + V2::new(0.05, 0.05));
            self.objects.push(cpy);
        }
        if let Some(ix) = self.selected_light {
            let mut cpy = self.lights[ix].clone();
            let pos = cpy.get_origin();
            cpy.set_origin(pos + V2::new(0.05, 0.05));
            self.lights.push(cpy);
        }
    }

    pub fn update_tick(&mut self, _frame_time: f64) {}

    pub fn get_render_to_texture(&self) -> bool {
        self.render_to_texture
    }

    pub fn set_render_to_texture(&mut self, render_to_texture: bool) {
        if render_to_texture != self.render_to_texture {
            println!("render_to_texture toggled: {}", render_to_texture);
            self.render_to_texture = render_to_texture;
            self.recreate_pipeline = true;
        }
    }

    pub fn get_trace_time(&self) -> f64 {
        self.trace_time_vd.iter().sum::<f64>() / self.trace_time_vd.len() as f64
    }

    pub fn trace_all_reflective(&mut self) -> Vec<(Vec<P2>, Color)> {
        if let Some(dro) = self.drawing_object.clone() {
            self.objects.push(dro);
        }
        let mut all_line_strips: Vec<(Vec<P2>, Color)> = Vec::new();
        for light in self.lights.iter() {
            let line_strips = light
                .get_rays()
                .par_iter()
                .map(|ray| {
                    let mut line_strip = vec![ray.get_origin()];
                    self.trace_reflective(&mut line_strip, ray, light.get_color(), self.max_bounce);
                    (line_strip, light.get_color())
                })
                .collect::<Vec<(Vec<P2>, Color)>>();
            all_line_strips.extend(line_strips);
        }
        if self.drawing_object.is_some() {
            self.objects.pop();
        }
        all_line_strips
    }

    pub fn trace_reflective(&self, rays: &mut Vec<P2>, ray: &Ray, color: Color, max_bounce: u32) {
        if max_bounce == 0 {
            return;
        }
        let mut refopt = None;
        let mut ret_intersect: Option<P2> = None;
        if let Some(intersection_point) = ray.intersect(&self.canvas_bounds) {
            ret_intersect = Some(intersection_point.get_first().0);
        }
        for obj in self.objects.iter() {
            if let Some(reflected) = ray.reflect_on(&obj.get_geometry()) {
                if let Some(intersect) = ret_intersect {
                    if distance(&ray.get_origin(), &reflected.get_origin())
                        < distance(&ray.get_origin(), &intersect)
                    {
                        ret_intersect = Some(reflected.get_origin());
                        refopt = Some(reflected);
                    }
                } else {
                    // first reflection
                    ret_intersect = Some(reflected.get_origin());
                    refopt = Some(reflected);
                }
            }
        }
        if let Some(ls) = ret_intersect {
            rays.push(ls);
        }
        if let Some(reflected) = refopt {
            self.trace_reflective(rays, &reflected, color, max_bounce - 1);
        }
    }

    pub fn trace_all(&mut self) -> Vec<(P2, Color)> {
        let instant_start = Instant::now();
        if let Some(dro) = self.drawing_object.as_ref() {
            self.objects.push(dro.clone());
        }
        if let Some(drl) = self.drawing_light.as_ref() {
            self.lights.push(drl.clone());
        }
        let mut all_lines: Vec<(P2, Color)> = Vec::new();
        for light in self.lights.iter() {
            let mut refractive_index = 1.;
            for obj in self.objects.iter() {
                if let Object::Circle(c, material) = obj {
                    if c.contains(&light.get_origin()) {
                        refractive_index = material.refractive_index;
                    }
                }
            }
            let lines: Vec<(P2, Color)> = light
                .get_rays()
                .par_chunks(self.chunk_size)
                .map(|rays| {
                    let mut lines = Vec::new();
                    for ray in rays {
                        self.trace(
                            &mut lines,
                            ray,
                            light.get_color(),
                            refractive_index,
                            self.max_bounce,
                        );
                    }
                    lines
                })
                .collect::<Vec<Vec<(P2, Color)>>>()
                .concat();
            all_lines.extend(lines);
        }
        if self.drawing_object.is_some() {
            self.objects.pop();
        }
        if self.drawing_light.is_some() {
            self.lights.pop();
        }

        // fill limit testing
        // all_lines.resize(
        // 1000,
        // (
        // LineSegment::from_ab(P2::new(0., 0.), P2::new(0., 0.)),
        // [1.0; 4],
        // ),
        // );

        // draw control lines for cubic bezier curves
        for obj in self.objects.iter() {
            if let Object::CurvedMirror(cm) = obj {
                let red = [1., 0., 0., 1.];
                all_lines.push((cm.cubic.points[0], red));
                all_lines.push((cm.cubic.points[1], red));
                all_lines.push((cm.cubic.points[1], red));
                all_lines.push((cm.cubic.points[2], red));
                all_lines.push((cm.cubic.points[2], red));
                all_lines.push((cm.cubic.points[3], red));
            }
        }

        self.trace_time_vd
            .push_back(instant_start.elapsed().as_micros() as f64 / 1000.0);
        if self.trace_time_vd.len() > 20 {
            self.trace_time_vd.pop_front();
        }

        all_lines
    }

    pub fn trace(
        &self,
        rays: &mut Vec<(P2, Color)>,
        ray: &Ray,
        color: Color,
        refractive_index: Float,
        max_bounce: u32,
    ) {
        let mut trace_rays = vec![(*ray, color, refractive_index)];
        let mut back_buffer = Vec::new();
        for _ in 0..max_bounce {
            if trace_rays.is_empty() {
                return;
            }
            for (ray, color, refractive_index) in &trace_rays {
                if (color[0] < self.cutoff_color[0]
                    && color[1] < self.cutoff_color[1]
                    && color[2] < self.cutoff_color[2])
                    || color[3] < self.cutoff_color[3]
                {
                    continue;
                }

                // find the nearest object
                let mut nearest: Float = std::f64::MAX;
                // (intersection point, normal, object index)
                let mut nearest_target: Option<(P2, Normal, usize)> = None;
                for (index, obj) in self.objects.iter().enumerate() {
                    if let Some(intersections) = ray.intersect(&obj.get_geometry()) {
                        for (intersection, normal) in intersections {
                            let dist_sq = distance_squared(&ray.get_origin(), &intersection);
                            if dist_sq < nearest {
                                nearest = dist_sq;
                                nearest_target = Some((intersection, normal, index));
                            }
                        }
                    }
                }

                if let Some((intersection, normal, index)) = nearest_target {
                    let obj = self.objects[index].clone();
                    match obj {
                        Object::Mirror(_) | Object::CurvedMirror(_) => {
                            rays.push((ray.get_origin(), *color));
                            rays.push((intersection, *color));
                            back_buffer.push((
                                ray.reflect(&intersection, &normal),
                                *color,
                                *refractive_index,
                            ));
                        }

                        Object::Rect(_, material)
                        | Object::Circle(_, material)
                        | Object::Lens(_, material)
                        | Object::Geo(_, material) => {
                            let mut updated_refractive_index = 1.;
                            let result;
                            if obj.contains(&ray.get_origin()) {
                                result = ray.refract(
                                    &intersection,
                                    &normal,
                                    material.refractive_index,
                                    1.,
                                );
                            } else {
                                updated_refractive_index = material.refractive_index;
                                result = ray.refract(
                                    &intersection,
                                    &normal,
                                    *refractive_index,
                                    material.refractive_index,
                                );
                            }
                            let (reflected, orefracted, reflectance) = result;
                            rays.push((ray.get_origin(), *color));
                            rays.push((reflected.get_origin(), *color));

                            let refl = reflectance as f32;
                            let omrefl = 1. - refl;
                            let color1 =
                                [color[0] * refl, color[1] * refl, color[2] * refl, color[3]];
                            let color2 = [
                                color[0] * omrefl,
                                color[1] * omrefl,
                                color[2] * omrefl,
                                color[3],
                            ];
                            back_buffer.push((reflected, color1, *refractive_index));
                            // self.trace(rays, &reflected, color1, refractive_index, max_bounce);
                            if let Some(refracted) = orefracted {
                                back_buffer.push((refracted, color2, updated_refractive_index));
                            }
                        }
                    }
                } else {
                    // handle canvas bounds
                    if let Some(canvas_intersect) = ray.intersect(&self.canvas_bounds) {
                        rays.push((ray.get_origin(), *color));
                        rays.push((canvas_intersect.get_first().0, *color));
                    }
                }
            }
            trace_rays.clear();
            trace_rays.extend(back_buffer.into_iter());
            back_buffer = Vec::new();
        }
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Mode {
    NoMode,
    Selecting(Option<LogicOp>),
    Selected,
    Move,
    Rotate,
    DrawMirrorStart,
    DrawMirrorEnd { start: P2 },
    DrawCircleStart,
    DrawCircleEnd { start: P2 },
    DrawRectStart,
    DrawRectEnd { start: P2 },
    DrawPointLight,
    DrawSpotLightStart,
    DrawSpotLightEnd { origin: P2 },
    DrawDirectionalLightStart,
    DrawDirectionalLightEnd { start: P2 },
}
