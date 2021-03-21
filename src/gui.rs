use crate::light_garden::*;
use collision2d::geo::*;
use egui::*;
use egui_winit_platform::{Platform, PlatformDescriptor};
use epi::*;
use instant::Instant;
use std::f64::consts::PI;
use wgpu::{BlendFactor, BlendOperation};

pub struct Gui {
    pub platform: Platform,
    pub scale_factor: f32,
    last_update_inst: Instant,
    last_cursor: Option<Pos2>,
    pub app: LightGarden,
    pub ui_mode: UiMode,
}

impl Gui {
    pub fn new(winit_window: &winit::window::Window, sc_desc: &wgpu::SwapChainDescriptor) -> Self {
        let size = winit_window.inner_size();
        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width,
            physical_height: size.height,
            scale_factor: winit_window.scale_factor(),
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });
        let last_update_inst = Instant::now();
        let app = LightGarden::new(
            collision2d::geo::Rect::from_tlbr(1., -1., -1., 1.),
            sc_desc.format,
        );

        Gui {
            platform,
            scale_factor: winit_window.scale_factor() as f32,
            last_update_inst,
            last_cursor: None,
            app,
            ui_mode: UiMode::new(),
        }
    }

    pub fn gui(&mut self) -> Vec<egui::ClippedMesh> {
        use egui::*;
        let elapsed = self.last_update_inst.elapsed();
        self.platform.begin_frame();
        let ctx = self.platform.context();
        if self.app.mode == Mode::NoMode || self.app.mode == Mode::Selected {
            let window = egui::Window::new("Light Garden");
            window
                .default_size(Vec2::new(300.0, 100.0))
                .show(&ctx, |ui| {
                    self.last_cursor = ui.input().pointer.interact_pos();
                    if let Some(mouse_pos) = self.last_cursor {
                        ui.label(format!(
                            "Mouse Position: ({:.1},{:.1})",
                            mouse_pos.x, mouse_pos.y
                        ));
                    }

                    match self.ui_mode {
                        UiMode::Main => {
                            self.main(ui);
                        }
                        UiMode::Add => {
                            self.add(ui);
                        }
                        UiMode::Selected => {
                            self.selected(ui);
                        }
                        UiMode::Settings => {
                            self.settings(ui);
                        }
                        UiMode::Grid => {
                            self.grid(ui);
                        }
                        UiMode::Exiting => {}
                    }

                    ui.label(format!("Frametime: {:?}", elapsed));
                    ui.label(format!("Average Trace Time: {}", self.app.get_trace_time()));
                });
        }

        self.last_update_inst = Instant::now();

        let (_output, paint_commands) = self.platform.end_frame();
        ctx.tessellate(paint_commands)
    }

    fn main(&mut self, ui: &mut Ui) {
        if ui.button("(A)dd ...").clicked() {
            self.ui_mode = UiMode::Add;
        }
        if ui.button("(S)elect").clicked() {
            self.ui_mode = UiMode::Selected;
            self.app.mode = Mode::Selecting(None);
        }
        if ui.button("S(e)ttings").clicked() {
            self.ui_mode = UiMode::Settings;
        }
        if ui.button("(G)rid").clicked() {
            self.ui_mode = UiMode::Grid;
        }
        if ui.button("(Q)it").clicked() {
            self.ui_mode = UiMode::Exiting;
        }
    }

    fn selected(&mut self, ui: &mut Ui) {
        if let Some(obj) = self.app.get_selected_object() {
            Gui::edit_object(obj, ui);
        }

        if ui.button("(C)opy").clicked() {
            self.app.copy_selected();
        }

        if ui.button("Mirror on (X) axis").clicked() {
            self.app.mirror_on_x_axis_selected();
        }

        if ui.button("Mirror on (Y) axis").clicked() {
            self.app.mirror_on_y_axis_selected();
        }

        if ui.button("(D)elete").clicked() {
            self.app.delete_selected();
            self.ui_mode = UiMode::Main;
        }

        self.edit(ui);

        if let Some(light) = self.app.get_selected_light() {
            Gui::edit_light(light, ui);
            if ui.button("(M)ove Light").clicked() {
                self.app.mode = Mode::Move;
            }
        } else {
            let ac = self.app.selected_color;
            let mut color =
                Color32::from(Rgba::from_rgba_premultiplied(ac[0], ac[1], ac[2], ac[3]));
            egui::widgets::color_picker::color_edit_button_srgba(
                ui,
                &mut color,
                color_picker::Alpha::OnlyBlend,
            );
            let rgba = Rgba::from(color);
            self.app.selected_color = [rgba[0], rgba[1], rgba[2], rgba[3]];
        }

        if let Some(Light::SpotLight(_)) = self.app.get_selected_light() {
            if ui.button("(R)otate").clicked() {
                self.app.mode = Mode::Rotate;
            }
        }
        if let Some(Light::DirectionalLight(_)) = self.app.get_selected_light() {
            if ui.button("(R)otate").clicked() {
                self.app.mode = Mode::Rotate;
            }
        }
    }

    fn add(&mut self, ui: &mut Ui) {
        if self.ui_mode == UiMode::Add {
            if ui.button("Add (P)oint Light").clicked() {
                self.app.mode = Mode::DrawPointLight;
            }
            if ui.button("Add (S)pot Light").clicked() {
                self.app.mode = Mode::DrawSpotLightStart;
            }
            if ui.button("Add (D)irectionalLight").clicked() {
                self.app.mode = Mode::DrawDirectionalLightStart;
            }
            if ui.button("Add (R)ect").clicked() {
                self.app.mode = Mode::DrawRectStart;
            }
            if ui.button("Add (C)ircle").clicked() {
                self.app.mode = Mode::DrawCircleStart;
            }
            if ui.button("Add (M)irror").clicked() {
                self.app.mode = Mode::DrawMirrorStart;
            }
        }
    }

    fn edit(&mut self, ui: &mut Ui) {
        if self.app.get_selected_object().is_some() {
            if ui.button("(M)ove Obj").clicked() {
                self.app.mode = Mode::Move;
            }
            if ui.button("(R)otate").clicked() {
                self.app.mode = Mode::Rotate;
            }
            if ui.button("(A)nd").clicked() {
                self.app.mode = Mode::Selecting(Some(LogicOp::And));
            }
            if ui.button("(O)r").clicked() {
                self.app.mode = Mode::Selecting(Some(LogicOp::Or));
            }
            if ui.button("And(N)ot").clicked() {
                self.app.mode = Mode::Selecting(Some(LogicOp::AndNot));
            }
        }
    }

    fn settings(&mut self, ui: &mut Ui) {
        let mut chunk_size = self.app.chunk_size as u32;
        ui.add(Slider::u32(&mut chunk_size, 1..=1000).text("Rayon Chunk Size"));
        self.app.chunk_size = chunk_size as usize;

        self.edit_blend(ui);

        self.edit_cutoff_color(ui);

        self.toggle_render_to_texture(ui);
    }

    fn grid(&mut self, ui: &mut Ui) {
        // toggle snap to grid
        ui.add(Checkbox::new(&mut self.app.grid.show, "Show grid"));
        ui.label("Snap to grid while pressing the left shift key");
        self.grid_size(ui);
        self.edit_grid_color(ui);
    }

    fn edit_light(light: &mut Light, ui: &mut Ui) {
        let mut update_light = false;
        match light {
            Light::PointLight(_point) => { /* no user interface elements to add */ }
            Light::SpotLight(spot) => {
                // spot angle
                let mut spot_angle = spot.spot_angle * 180. / PI;
                let old_spot_angle = spot_angle;
                ui.add(Slider::f64(&mut spot_angle, 0.0..=360.0).text("Spot Angle"));
                if spot_angle != old_spot_angle {
                    spot.spot_angle = spot_angle * PI / 180.;
                    update_light = true;
                }
            }
            Light::DirectionalLight(_direction) => {}
        }

        // num rays
        let mut num_rays_mut = light.get_num_rays();
        let num_rays = num_rays_mut;
        ui.add(Slider::u32(&mut num_rays_mut, 1..=30000).text("Num Rays"));
        if num_rays != num_rays_mut || update_light {
            light.set_num_rays(num_rays_mut);
        }

        // light color
        let lc = light.get_color();
        let mut color = Color32::from(Rgba::from_rgba_premultiplied(lc[0], lc[1], lc[2], lc[3]));
        egui::widgets::color_picker::color_edit_button_srgba(
            ui,
            &mut color,
            color_picker::Alpha::OnlyBlend,
        );
        let rgba = Rgba::from(color);
        light.set_color(rgba[0], rgba[1], rgba[2], rgba[3]);
    }

    fn edit_grid_color(&mut self, ui: &mut Ui) {
        let c = self.app.grid.get_color();
        let mut color = Color32::from(Rgba::from_rgba_premultiplied(c[0], c[1], c[2], c[3]));
        egui::widgets::color_picker::color_edit_button_srgba(
            ui,
            &mut color,
            color_picker::Alpha::OnlyBlend,
        );
        let rgba = Rgba::from(color);
        self.app
            .grid
            .set_color([rgba[0], rgba[1], rgba[2], rgba[3]]);
    }

    fn edit_object(object: &mut Object, ui: &mut Ui) {
        if let Some(material) = object.material_mut() {
            let mut whole: i32 = material.refractive_index.floor() as i32;
            let mut frac: Float = material.refractive_index - whole as Float;
            ui.add(Slider::i32(&mut whole, -10..=10).text("Refractive Index whole part"));
            ui.add(Slider::f64(&mut frac, -0.0..=0.999).text("Refractive Index fractional part"));
            material.refractive_index = whole as Float + frac;
        }
    }

    fn edit_blend(&mut self, ui: &mut Ui) {
        ui.add(Slider::u32(&mut self.app.max_bounce, 1..=12).text("Max Bounce:"));
        let blend_factors: &[BlendFactor] = &[
            BlendFactor::Zero,
            BlendFactor::One,
            BlendFactor::OneMinusSrcColor,
            BlendFactor::OneMinusSrcAlpha,
            BlendFactor::OneMinusDstColor,
            BlendFactor::OneMinusDstAlpha,
            BlendFactor::DstColor,
            BlendFactor::DstAlpha,
            BlendFactor::SrcAlpha,
            BlendFactor::SrcColor,
            BlendFactor::SrcAlphaSaturated,
            BlendFactor::BlendColor,
            BlendFactor::OneMinusBlendColor,
        ];
        let mut selected_changed = false;

        let mut selected = self.app.color_state_descriptor.color_blend.src_factor;
        combo_box_with_label(ui, "ColorSrc", format!("{:?}", selected), |ui| {
            blend_factors.iter().for_each(|bf| {
                ui.selectable_value(&mut selected, *bf, format!("{:?}", bf));
            });
        });
        selected_changed |= self.app.color_state_descriptor.color_blend.src_factor != selected;
        self.app.color_state_descriptor.color_blend.src_factor = selected;

        selected = self.app.color_state_descriptor.color_blend.dst_factor;
        combo_box_with_label(ui, "ColorDst", format!("{:?}", selected), |ui| {
            blend_factors.iter().for_each(|bf| {
                ui.selectable_value(&mut selected, *bf, format!("{:?}", bf));
            });
        });
        selected_changed |= self.app.color_state_descriptor.color_blend.dst_factor != selected;
        self.app.color_state_descriptor.color_blend.dst_factor = selected;

        selected = self.app.color_state_descriptor.alpha_blend.src_factor;
        combo_box_with_label(ui, "AlphaSrc", format!("{:?}", selected), |ui| {
            blend_factors.iter().for_each(|bf| {
                ui.selectable_value(&mut selected, *bf, format!("{:?}", bf));
            });
        });
        selected_changed |= self.app.color_state_descriptor.alpha_blend.src_factor != selected;
        self.app.color_state_descriptor.alpha_blend.src_factor = selected;

        selected = self.app.color_state_descriptor.alpha_blend.dst_factor;
        combo_box_with_label(ui, "AlphaDst", format!("{:?}", selected), |ui| {
            blend_factors.iter().for_each(|bf| {
                ui.selectable_value(&mut selected, *bf, format!("{:?}", bf));
            });
        });
        selected_changed |= self.app.color_state_descriptor.alpha_blend.dst_factor != selected;
        self.app.color_state_descriptor.alpha_blend.dst_factor = selected;

        let blend_ops: &[BlendOperation] = &[
            BlendOperation::Min,
            BlendOperation::Max,
            BlendOperation::Add,
            BlendOperation::Subtract,
            BlendOperation::ReverseSubtract,
        ];

        let mut selected = self.app.color_state_descriptor.color_blend.operation;
        combo_box_with_label(ui, "BlendOpColor", format!("{:?}", selected), |ui| {
            blend_ops.iter().for_each(|bf| {
                ui.selectable_value(&mut selected, *bf, format!("{:?}", bf));
            });
        });
        selected_changed |= self.app.color_state_descriptor.color_blend.operation != selected;
        self.app.color_state_descriptor.color_blend.operation = selected;

        selected = self.app.color_state_descriptor.alpha_blend.operation;
        combo_box_with_label(ui, "BlendOpAlpha", format!("{:?}", selected), |ui| {
            blend_ops.iter().for_each(|bf| {
                ui.selectable_value(&mut selected, *bf, format!("{:?}", bf));
            });
        });
        selected_changed |= self.app.color_state_descriptor.alpha_blend.operation != selected;
        self.app.color_state_descriptor.alpha_blend.operation = selected;

        self.app.recreate_pipeline |= selected_changed;
    }

    pub fn edit_cutoff_color(&mut self, ui: &mut Ui) {
        let mut color = self.app.cutoff_color;
        let mut rgb = (color[0] + color[1] + color[2]) / 3.;
        ui.add(Slider::f32(&mut rgb, 0.00001..=0.05).text("Cutoff RGB"));
        color[0] = rgb;
        color[1] = rgb;
        color[2] = rgb;
        ui.add(Slider::f32(&mut color[3], 0.00001..=0.05).text("Cutoff Alpha"));
        self.app.cutoff_color = color;
    }

    pub fn toggle_render_to_texture(&mut self, ui: &mut Ui) {
        let mut render_to_texture = self.app.get_render_to_texture();
        ui.add(Checkbox::new(&mut render_to_texture, "render to texture"));
        self.app.set_render_to_texture(render_to_texture);
    }

    pub fn grid_size(&mut self, ui: &mut Ui) {
        let mut grid_size = self.app.grid.get_dist();
        ui.add(Slider::f64(&mut grid_size, 0.01..=0.1).text("Grid size"));
        self.app
            .grid
            .set_dist(grid_size, &self.app.get_canvas_bounds());
    }

    pub fn update(
        &mut self,
        event: &winit::event::WindowEvent,
        sc_desc: &wgpu::SwapChainDescriptor,
    ) {
        use winit::event;
        use winit::event::WindowEvent;
        type Key = event::VirtualKeyCode;
        match event {
            winit::event::WindowEvent::KeyboardInput { input, .. } => {
                if winit::event::ElementState::Released == input.state {
                    match (input.virtual_keycode, self.ui_mode) {
                        (Some(Key::Escape), ui_mode) => match ui_mode {
                            UiMode::Main => {}
                            UiMode::Add => self.ui_mode = UiMode::Main,
                            UiMode::Selected => {
                                self.app.deselect();
                                self.ui_mode = UiMode::Main;
                            }
                            UiMode::Settings => self.ui_mode = UiMode::Main,
                            UiMode::Grid => self.ui_mode = UiMode::Main,
                            UiMode::Exiting => {}
                        },
                        (Some(Key::A), UiMode::Main) => self.ui_mode = UiMode::Add,
                        (Some(Key::S), UiMode::Main) => {
                            self.ui_mode = UiMode::Selected;
                            self.app.mode = Mode::Selecting(None)
                        }
                        (Some(Key::E), UiMode::Main) => self.ui_mode = UiMode::Settings,

                        (Some(Key::P), UiMode::Add) => self.app.mode = Mode::DrawPointLight,
                        (Some(Key::S), UiMode::Add) => self.app.mode = Mode::DrawSpotLightStart,
                        (Some(Key::D), UiMode::Add) => {
                            self.app.mode = Mode::DrawDirectionalLightStart
                        }

                        (Some(Key::R), UiMode::Add) => self.app.mode = Mode::DrawRectStart,
                        (Some(Key::C), UiMode::Add) => self.app.mode = Mode::DrawCircleStart,
                        (Some(Key::M), UiMode::Add) => self.app.mode = Mode::DrawMirrorStart,

                        (Some(Key::M), UiMode::Selected) => self.app.mode = Mode::Move,
                        (Some(Key::R), UiMode::Selected) => self.app.mode = Mode::Rotate,
                        (Some(Key::A), UiMode::Selected) => {
                            self.app.mode = Mode::Selecting(Some(LogicOp::And))
                        }
                        (Some(Key::O), UiMode::Selected) => {
                            self.app.mode = Mode::Selecting(Some(LogicOp::Or))
                        }
                        (Some(Key::N), UiMode::Selected) => {
                            self.app.mode = Mode::Selecting(Some(LogicOp::AndNot))
                        }
                        (Some(Key::D), UiMode::Selected) => {
                            self.app.delete_selected();
                            self.ui_mode = UiMode::Main;
                        }
                        (Some(Key::C), UiMode::Selected) => self.app.copy_selected(),
                        (Some(Key::X), UiMode::Selected) => self.app.mirror_on_x_axis_selected(),
                        (Some(Key::Y), UiMode::Selected) => self.app.mirror_on_y_axis_selected(),

                        (Some(Key::Q), _) => self.ui_mode = UiMode::Exiting,

                        _ => {}
                    }
                }
                if let (Some(Key::LShift), winit::event::ElementState::Pressed) =
                    (input.virtual_keycode, input.state)
                {
                    self.app.grid.on = true;
                } else {
                    self.app.grid.on = false;
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let aspect = sc_desc.width as f64 / sc_desc.height as f64;
                self.app.update_mouse_position(nalgebra::Point2::new(
                    ((2. * position.x / (sc_desc.width as f64)) - 1.) * aspect,
                    (2. * -position.y / (sc_desc.height as f64)) + 1.,
                ));
            }
            WindowEvent::MouseInput {
                state: event::ElementState::Released,
                button: event::MouseButton::Left,
                ..
            } => {
                self.app.mouse_clicked();
            }
            WindowEvent::MouseInput {
                state: event::ElementState::Released,
                button: event::MouseButton::Right,
                ..
            } => {
                self.app.deselect();
                self.ui_mode = UiMode::Main;
            }
            _ => {}
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum UiMode {
    Main,
    Add,
    Selected,
    Settings,
    Grid,
    Exiting,
}

impl UiMode {
    pub fn new() -> UiMode {
        UiMode::Main
    }
}
