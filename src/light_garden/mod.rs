extern crate nalgebra as na;

use collision2d::geo::*;
pub use drawer::*;
use grid::Grid;
use instant::Instant;
pub use light::*;
use na::{distance, Point2};
pub use object::*;
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;
use std::{collections::VecDeque, f64::consts::*};
pub use string_mod::*;
pub use tile_map::*;
pub use tracer::*;
use wgpu::BlendState;

pub mod drawer;
pub mod grid;
pub mod light;
pub mod object;
pub mod string_mod;
pub mod tile_map;
pub mod tracer;
/// the maximum from an objects/lights origin at which a DragEvent can move it
const MOVE_DIST: Float = 0.2;

pub struct LightGarden {
    pub tracer: Tracer,
    pub drawer: Drawer,
    mouse_pos: Point2<Float>,
    pub num_rays: u32,
    pub selected_object: Option<usize>,
    pub selected_light: Option<usize>,
    pub selected_color: Color,
    pub color_state_descriptor: wgpu::ColorTargetState,
    pub recreate_pipeline: bool,
    pub ray_width: f64,
    pub mode: Mode,
    render_to_texture: bool,
    pub string_mods: Vec<StringMod>,
    pub string_mod_ix: usize,
    pub screenshot_path: Option<String>,
    drag_radius: f64,
    mouse_is_down: bool,
    initial_mouse_down: P2,
    drag_event: Option<DragEvent>,
}

impl LightGarden {
    pub fn new(canvas_bounds: Rect, descriptor_format: wgpu::TextureFormat) -> LightGarden {
        let color_state_descriptor = wgpu::ColorTargetState {
            format: descriptor_format,
            blend: Some(BlendState {
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            }),
            write_mask: wgpu::ColorWrites::ALL,
        };
        LightGarden {
            tracer: Tracer::new(&canvas_bounds),
            drawer: Drawer::new(&canvas_bounds),
            mouse_pos: P2::new(0., 0.),
            selected_object: None,
            selected_light: None,
            color_state_descriptor,
            recreate_pipeline: true,
            num_rays: 2000,
            selected_color: [0.02, 0.03, 0.05, 0.01],
            render_to_texture: false,
            ray_width: 1.0,
            mode: Mode::Selecting(None),
            string_mods: vec![StringMod::new()],
            string_mod_ix: 0,
            screenshot_path: None,
            drag_radius: 0.05,
            mouse_is_down: false,
            initial_mouse_down: P2::new(0., 0.),
            drag_event: None,
        }
    }

    pub fn update_mouse_position(&mut self, position: P2) {
        self.mouse_pos = position;
        self.tracer.grid.snap_to_grid(&mut self.mouse_pos);
        if !self.tracer.canvas_bounds.contains(&self.mouse_pos) {
            return;
        }
        if self.mouse_is_down {
            if let Some(drag_event) = self.drag_event.as_mut() {
                drag_event.end = self.mouse_pos;
            } else {
                let dist = distance(&self.mouse_pos, &self.initial_mouse_down);
                if dist > self.drag_radius {
                    self.drag_event = Some(DragEvent {
                        start: self.initial_mouse_down,
                        end: self.mouse_pos,
                    });
                }
            }
            self.mouse_dragged();
        }
    }

    pub fn update(&mut self) {
        match &self.mode {
            Mode::DrawMirrorEnd { start } => {
                self.tracer
                    .add_drawing_object(Object::new_mirror(*start, self.mouse_pos));
            }

            Mode::DrawCircleEnd { start } => {
                self.tracer.add_drawing_object(Object::new_circle(
                    *start,
                    distance(start, &self.mouse_pos),
                ));
            }

            Mode::DrawRectEnd { start } => {
                let vdiff_t2 = 2. * (self.mouse_pos - start);
                let width = vdiff_t2[0].abs();
                let height = vdiff_t2[1].abs();
                self.tracer
                    .add_drawing_object(Object::new_rect(*start, width, height));
            }

            Mode::DrawPointLight => {
                self.tracer
                    .add_drawing_light(Light::PointLight(PointLight::new(
                        self.mouse_pos,
                        self.num_rays,
                        self.selected_color,
                    )));
            }

            Mode::DrawSpotLightStart => {
                self.tracer
                    .add_drawing_light(Light::SpotLight(SpotLight::new(
                        self.mouse_pos,
                        FRAC_PI_4,
                        V2::new(1., 0.),
                        self.num_rays,
                        self.selected_color,
                    )));
            }

            Mode::DrawSpotLightEnd { origin } => {
                self.tracer
                    .add_drawing_light(Light::SpotLight(SpotLight::new(
                        *origin,
                        FRAC_PI_4,
                        self.mouse_pos - origin,
                        self.num_rays,
                        self.selected_color,
                    )));
            }

            Mode::DrawDirectionalLightEnd { start } => {
                self.tracer
                    .add_drawing_light(Light::DirectionalLight(DirectionalLight::new(
                        self.selected_color,
                        self.num_rays,
                        LineSegment::from_ab(*start, self.mouse_pos),
                    )));
            }

            Mode::Rotate => {
                let mouse_pos = self.mouse_pos;
                if let Some(obj) = self.get_selected_object() {
                    obj.y_axis_look_at(&mouse_pos);
                }
                if let Some(Light::SpotLight(spot)) = self.get_selected_light() {
                    spot.x_axis_look_at(&mouse_pos);
                }
                if let Some(Light::DirectionalLight(directional_light)) = self.get_selected_light()
                {
                    directional_light.y_axis_look_at(&mouse_pos);
                }
            }

            Mode::SelectTile => {
                if let Some(tile) = self.tracer.get_tile(&self.mouse_pos) {
                    self.drawer.draw_aabb(&tile.aabb, [1.0, 0.0, 0.0, 1.0]);
                    for obj in self.tracer.object_iterator() {
                        if let Some((lsa, lsb)) = tile.aabb.get_crossover(&obj.get_aabb()) {
                            self.drawer.draw_aabb(&obj.get_aabb(), [0.5, 0.0, 0.0, 1.0]);
                            self.drawer.draw_line_segment(&lsa, [0.0, 0.5, 0.5, 1.0]);
                            self.drawer.draw_line_segment(&lsb, [0.5, 0.5, 0.0, 1.0]);
                        }
                    }
                }
            }

            Mode::TileSelected { tile } => {
                let slab = tile.index(&Unit::new_normalize(
                    self.mouse_pos - tile.aabb.get_origin(),
                ));
                self.drawer.draw_aabb(&tile.aabb, [1.0, 0.0, 0.0, 1.0]);
                self.drawer.draw_geo(slab.rleft, [0.0, 1.0, 0.0, 1.0]);
                self.drawer.draw_geo(slab.rright, [0.0, 0.0, 1.0, 1.0]);
                let mut collision_points = Vec::new();
                for obj in self.tracer.object_iterator() {
                    if let Some(mut points) = slab.rleft.intersect(&obj.get_geometry()) {
                        collision_points.append(&mut points);
                    }
                    if let Some(mut points) = slab.rright.intersect(&obj.get_geometry()) {
                        collision_points.append(&mut points);
                    }
                    if let Some(points) = slab.rleft.intersect(&obj.get_geometry().get_aabb()) {
                        collision_points.append(&mut points.to_vec());
                    }
                    if let Some(points) = slab.rright.intersect(&obj.get_geometry().get_aabb()) {
                        collision_points.append(&mut points.to_vec());
                    }
                }
                for point in collision_points {
                    self.drawer.draw_point(&point.0, [1.0, 0.0, 1.0, 1.0]);
                }
            }

            _ => {}
        }
    }

    pub fn get_canvas_bounds(&self) -> Rect {
        self.tracer.canvas_bounds
    }

    pub fn mouse_down(&mut self) {
        self.mouse_is_down = true;
        self.initial_mouse_down = self.mouse_pos;
    }

    pub fn mouse_released(&mut self) {
        self.mouse_is_down = false;
        if self.drag_event.is_some() {
            self.mouse_dragged();
        } else {
            self.mouse_clicked();
        }
        self.drag_event = None;
        if self.mode == Mode::Moving {
            self.mode = Mode::Selected;
        }
    }

    fn mouse_clicked(&mut self) {
        if !self.tracer.canvas_bounds.contains(&self.mouse_pos) {
            return;
        }
        match &mut self.mode {
            Mode::NoMode => {
                self.selected_object = None;
                self.selected_light = None;
            }

            Mode::Selecting(None) => {
                self.selected_object = None;
                self.selected_light = None;
                let mut min_distance = Float::MAX;
                for (ix, o) in self.tracer.object_iterator().enumerate() {
                    let dist = o.distance(&self.mouse_pos);
                    if dist < min_distance {
                        min_distance = dist;
                        self.selected_object = Some(ix);
                    }
                }
                for (ix, l) in self.tracer.light_iterator().enumerate() {
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
                    for (ix, o) in self.tracer.object_iterator().enumerate() {
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
                            let geo_a = self.tracer.index_object(current_ix).get_geometry();
                            let geo_b = self.tracer.index_object(click_ix).get_geometry();
                            let geo = match op {
                                LogicOp::And => geo_a & geo_b,
                                LogicOp::Or => geo_a | geo_b,
                                LogicOp::AndNot => geo_a.and_not(geo_b),
                            };
                            self.tracer
                                .replace_object(current_ix.min(click_ix), Object::new_geo(geo));
                            // current_ix != click_ix
                            self.tracer.remove_object(current_ix.max(click_ix));
                            self.mode = Mode::Selected;
                            self.selected_object = Some(current_ix.min(click_ix));
                        }
                    }
                } else {
                    // there is no currently selected object so performing a logic op is not possible
                    self.mode = Mode::Selecting(None);
                }
            }

            Mode::Selected => {}

            Mode::Moving => {}

            Mode::Rotate => {
                self.mode = Mode::Selected;
            }

            Mode::EditObject => {}

            Mode::DrawPointLight => {
                self.tracer
                    .add_drawing_light(Light::PointLight(PointLight::new(
                        self.mouse_pos,
                        self.num_rays,
                        self.selected_color,
                    )));
                self.tracer.finish_drawing_light(false);
                self.mode = Mode::Selecting(None);
            }

            Mode::DrawSpotLightStart => {
                self.mode = Mode::DrawSpotLightEnd {
                    origin: self.mouse_pos,
                };
            }

            Mode::DrawSpotLightEnd { origin } => {
                self.tracer
                    .add_drawing_light(Light::SpotLight(SpotLight::new(
                        *origin,
                        std::f64::consts::FRAC_PI_4,
                        self.mouse_pos - *origin,
                        self.num_rays,
                        self.selected_color,
                    )));
                self.tracer.finish_drawing_light(false);
                self.mode = Mode::Selecting(None);
            }

            Mode::DrawDirectionalLightStart => {
                self.mode = Mode::DrawDirectionalLightEnd {
                    start: self.mouse_pos,
                };
            }

            Mode::DrawDirectionalLightEnd { start } => {
                self.tracer
                    .add_drawing_light(Light::DirectionalLight(DirectionalLight::new(
                        self.selected_color,
                        self.num_rays,
                        LineSegment::from_ab(*start, self.mouse_pos),
                    )));
                self.tracer.finish_drawing_light(false);
                self.mode = Mode::Selecting(None);
            }

            Mode::DrawMirrorStart => {
                self.mode = Mode::DrawMirrorEnd {
                    start: self.mouse_pos,
                };
            }

            Mode::DrawMirrorEnd { start } => {
                self.tracer
                    .add_drawing_object(Object::new_mirror(*start, self.mouse_pos));
                self.tracer.finish_drawing_object(false);
                self.tracer.drawing_object_changed();
                self.mode = Mode::Selecting(None);
            }

            Mode::DrawCircleStart => {
                self.mode = Mode::DrawCircleEnd {
                    start: self.mouse_pos,
                };
            }

            Mode::DrawCircleEnd { start } => {
                self.tracer.add_drawing_object(Object::new_circle(
                    *start,
                    distance(start, &self.mouse_pos),
                ));
                self.tracer.drawing_object_changed();
                self.tracer.finish_drawing_object(false);
                self.mode = Mode::Selecting(None);
            }

            Mode::DrawRectStart => {
                self.mode = Mode::DrawRectEnd {
                    start: self.mouse_pos,
                };
            }

            Mode::DrawRectEnd { ref start } => {
                let vdiff_t2 = 2. * (self.mouse_pos - start);
                let width = vdiff_t2[0].abs();
                let height = vdiff_t2[1].abs();
                self.tracer
                    .add_drawing_object(Object::new_rect(*start, width, height));
                self.tracer.drawing_object_changed();
                self.tracer.finish_drawing_object(false);
                self.mode = Mode::Selecting(None);
            }

            Mode::DrawConvexPolygon { ref mut points } => {
                points.push(self.mouse_pos);
                if points.len() > 2 {
                    self.tracer
                        .add_drawing_object(Object::new_convex_polygon(points));
                }
            }

            Mode::SelectTile => {
                if let Some(tile) = self.tracer.get_tile(&self.mouse_pos) {
                    self.mode = Mode::TileSelected { tile: tile.clone() };
                }
            }

            Mode::TileSelected { .. } => {}

            Mode::StringMod => {}
        }
    }

    fn mouse_dragged(&mut self) {
        if let Some(drag_event) = self.drag_event {
            match self.mode {
                Mode::Selected => {
                    if let Some(obj) = self.get_selected_object() {
                        if distance(&obj.get_origin(), &drag_event.start) < MOVE_DIST {
                            self.mode = Mode::Moving;
                        }
                    }
                    if let Some(light) = self.get_selected_light() {
                        if distance(&light.get_origin(), &drag_event.start) < MOVE_DIST {
                            self.mode = Mode::Moving;
                        }
                    }
                }
                Mode::Moving => {
                    if let Some(obj) = self.get_selected_object() {
                        obj.set_origin(drag_event.end);
                    }
                    if let Some(light) = self.get_selected_light() {
                        light.set_origin(drag_event.end);
                    }
                }
                Mode::EditObject => {
                    if let Some(obj) = self.get_selected_object() {
                        match obj.object_enum {
                            ObjectE::CurvedMirror(ref mut cm) => {
                                println!("dragged on edit curved mirror");
                                let mut min_distance = Float::MAX;
                                let mut min_ix = 0;
                                for (ix, point) in cm.cubic.points.iter().enumerate() {
                                    let dist = distance(point, &drag_event.end);
                                    if dist < min_distance {
                                        min_distance = dist;
                                        min_ix = ix;
                                    }
                                }
                                cm.cubic.points[min_ix] = drag_event.end;
                            }
                            ObjectE::Circle(_c) => {}
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn clear(&mut self) {
        self.tracer.clear();
        self.selected_object = None;
        self.selected_light = None;
    }

    pub fn get_selected_object(&mut self) -> Option<&mut Object> {
        if let Some(ix) = self.selected_object {
            if self.tracer.tile_map_enabled() {
                self.tracer.obj_changed(ix);
            }
            let obj_ref = self.tracer.index_object(ix);
            Some(obj_ref)
        } else {
            None
        }
    }

    pub fn get_selected_light(&mut self) -> Option<&mut Light> {
        if let Some(ix) = self.selected_light {
            let light_ref = self.tracer.index_light(ix);
            Some(light_ref)
        } else {
            None
        }
    }

    pub fn delete_selected(&mut self) {
        if let Some(ix) = self.selected_light {
            self.tracer.remove_light(ix);
        }
        if let Some(ix) = self.selected_object {
            self.tracer.remove_object(ix);
        }
        self.deselect();
    }

    pub fn deselect(&mut self) {
        self.selected_light = None;
        self.selected_object = None;
        self.tracer.finish_drawing_object(true);
        self.tracer.finish_drawing_light(true);
    }

    pub fn copy_selected(&mut self) {
        if let Some(ix) = self.selected_object {
            let mut cpy = self.tracer.index_object(ix).clone();
            let pos = cpy.get_origin();
            cpy.set_origin(pos + V2::new(0.05, 0.05));
            self.tracer.push_object(cpy);
        }
        if let Some(ix) = self.selected_light {
            let mut cpy = self.tracer.index_light(ix).clone();
            let pos = cpy.get_origin();
            cpy.set_origin(pos + V2::new(0.05, 0.05));
            self.tracer.push_light(cpy);
        }
    }

    pub fn mirror_on_x_axis_selected(&mut self) {
        if let Some(ix) = self.selected_object {
            let obj = self.tracer.index_object(ix).mirror_y();
            self.tracer.push_object(obj);
        }
    }

    pub fn mirror_on_y_axis_selected(&mut self) {
        if let Some(ix) = self.selected_object {
            let obj = self.tracer.index_object(ix).mirror_x();
            self.tracer.push_object(obj);
        }
    }

    pub fn get_mouse_pos(&self) -> P2 {
        self.mouse_pos
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

    pub fn draw(&mut self) -> Vec<(P2, Color)> {
        if self.mode == Mode::StringMod {
            let mut res = Vec::new();
            for s in &self.string_mods {
                res.append(&mut s.draw());
            }
            res
        } else {
            let mut lines = self.tracer.trace_all();
            lines.extend(self.drawer.get_lines());
            lines
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum Mode {
    NoMode,
    Selecting(Option<LogicOp>),
    Selected,
    Moving,
    Rotate,
    EditObject,
    DrawMirrorStart,
    DrawMirrorEnd { start: P2 },
    DrawCircleStart,
    DrawCircleEnd { start: P2 },
    DrawRectStart,
    DrawRectEnd { start: P2 },
    DrawConvexPolygon { points: Vec<P2> },
    DrawPointLight,
    DrawSpotLightStart,
    DrawSpotLightEnd { origin: P2 },
    DrawDirectionalLightStart,
    DrawDirectionalLightEnd { start: P2 },
    SelectTile,
    TileSelected { tile: Tile },
    StringMod,
}

use std::fmt::{Display, Formatter, Result};

impl Display for Mode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::NoMode => write!(f, "NoMode"),
            Self::Selecting(None) => write!(f, "Selecting(None)"),
            Self::Selecting(Some(op)) => write!(f, "Selecting({:?})", op),
            Self::Selected => write!(f, "Selected"),
            Self::Moving => write!(f, "Moving"),
            Self::Rotate => write!(f, "Rotate"),
            Self::EditObject => write!(f, "EditObject"),
            Self::DrawMirrorStart => write!(f, "DrawMirrorStart"),
            Self::DrawMirrorEnd { start: _ } => write!(f, "DrawMirrorEnd"),
            Self::DrawCircleStart => write!(f, "DrawCircleStart"),
            Self::DrawCircleEnd { .. } => write!(f, "DrawCircleEnd"),
            Self::DrawRectStart => write!(f, "DrawRectStart"),
            Self::DrawRectEnd { .. } => write!(f, "DrawRectEnd"),
            Self::DrawConvexPolygon { .. } => write!(f, "DrawConvexPolygon"),
            Self::DrawPointLight => write!(f, "DrawPointLight"),
            Self::DrawSpotLightStart => write!(f, "DrawSpotLightStart"),
            Self::DrawSpotLightEnd { .. } => write!(f, "DrawSpotLightEnd"),
            Self::DrawDirectionalLightStart => write!(f, "DrawDirectionalLightStart"),
            Self::DrawDirectionalLightEnd { .. } => write!(f, "DrawDirectionalLightEnd"),
            Self::SelectTile => write!(f, "SelectTile"),
            Self::TileSelected { .. } => write!(f, "TileSelected"),
            Self::StringMod => write!(f, "StringMod"),
        }
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct DragEvent {
    start: P2,
    end: P2,
}
