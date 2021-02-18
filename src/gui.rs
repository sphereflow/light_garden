use crate::light_garden::*;
use collision2d::geo::*;
use egui::*;
use egui_winit_platform::{Platform, PlatformDescriptor};
use epi::*;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
use wgpu::{BlendFactor, BlendOperation};

pub struct Gui {
    pub platform: Platform,
    pub scale_factor: f32,
    #[cfg(not(target_arch = "wasm32"))]
    last_update_inst: Instant,
    last_cursor: Option<Pos2>,
    pub app: LightGarden,
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
        #[cfg(not(target_arch = "wasm32"))]
        let last_update_inst = Instant::now();
        let app = LightGarden::new(
            collision2d::geo::Rect::from_tlbr(1., -1., -1., 1.),
            sc_desc.format,
        );

        Gui {
            platform,
            scale_factor: winit_window.scale_factor() as f32,
            #[cfg(not(target_arch = "wasm32"))]
            last_update_inst,
            last_cursor: None,
            app,
        }
    }

    pub fn gui(&mut self) -> Vec<egui::ClippedMesh> {
        use egui::*;
        #[cfg(not(target_arch = "wasm32"))]
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

                    if ui.button("Add Point Light").clicked() {
                        self.app.mode = Mode::DrawPointLight;
                    }
                    if ui.button("Add Rect").clicked() {
                        self.app.mode = Mode::DrawRectStart;
                    }
                    if ui.button("Add Circle").clicked() {
                        self.app.mode = Mode::DrawCircleStart;
                    }
                    if ui.button("Select").clicked() {
                        self.app.mode = Mode::Selecting(None);
                    }

                    let mut chunk_size = self.app.chunk_size as u32;
                    ui.add(
                        Slider::u32(&mut chunk_size, 1..=1000).text(format!("Rayon Chunk Size")),
                    );
                    self.app.chunk_size = chunk_size as usize;

                    if let Some(obj) = self.app.get_selected_object() {
                        Gui::edit_object(obj, ui);
                    }
                    if self.app.get_selected_object().is_some() {
                        if ui.button("Move Obj").clicked() {
                            self.app.mode = Mode::Move;
                        }
                        if ui.button("Rotate").clicked() {
                            self.app.mode = Mode::Rotate;
                        }
                        if ui.button("And").clicked() {
                            self.app.mode = Mode::Selecting(Some(LogicOp::And));
                        }
                        if ui.button("Or").clicked() {
                            self.app.mode = Mode::Selecting(Some(LogicOp::Or));
                        }
                        if ui.button("AndNot").clicked() {
                            self.app.mode = Mode::Selecting(Some(LogicOp::AndNot));
                        }
                    }
                    if let Some(obj_ix) = self.app.selected_object {
                        ui.label(format!("Selected Object Index: {}", obj_ix));
                        if ui.button("Delete").clicked() {
                            self.app.delete_selected();
                        }
                    }
                    if let Some(ligh_ix) = self.app.selected_light {
                        ui.label(format!("Selected Light Index: {}", ligh_ix));
                        if ui.button(format!("Delete")).clicked() {
                            self.app.delete_selected();
                        }
                    }
                    if let Some(light) = self.app.get_selected_light() {
                        Gui::edit_light(light, ui);
                        if ui.button(format!("Move Light")).clicked() {
                            self.app.mode = Mode::Move;
                        }
                    } else {
                        let ac = self.app.selected_color;
                        let mut color = Color32::from(Rgba::from_rgba_premultiplied(
                            ac[0], ac[1], ac[2], ac[3],
                        ));
                        egui::widgets::color_picker::color_edit_button_srgba(
                            ui,
                            &mut color,
                            color_picker::Alpha::OnlyBlend,
                        );
                        let rgba = Rgba::from(color);
                        self.app.selected_color = [rgba[0], rgba[1], rgba[2], rgba[3]];
                    }

                    self.edit_blend(ui);

                    self.edit_cutoff_color(ui);

                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        ui.label(format!("Frametime: {:?}", elapsed));
                        ui.label(format!("Average Trace Time: {}", self.app.get_trace_time()));
                    }
                });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.last_update_inst = Instant::now();
        }

        let (_output, paint_commands) = self.platform.end_frame();
        ctx.tessellate(paint_commands)
    }

    fn edit_light(light: &mut Light, ui: &mut Ui) {
        let mut num_rays_mut = light.get_num_rays();
        let num_rays = num_rays_mut;
        ui.add(Slider::u32(&mut num_rays_mut, 1..=10000).text("Num Rays"));
        if num_rays != num_rays_mut {
            light.set_num_rays(num_rays_mut);
        }
        let lc = light.get_color();
        let mut color = Color32::from(Rgba::from_rgba_premultiplied(lc[0], lc[1], lc[2], lc[3]));
        egui::widgets::color_picker::color_edit_button_srgba(
            ui,
            &mut color,
            color_picker::Alpha::OnlyBlend,
        );
        let rgba = Rgba::from(color);
        light.set_color(rgba[0], rgba[1], rgba[2], rgba[3]);

        match light {
            Light::PointLight(_point) => { /* no user interface elements to add */ }
            Light::SpotLight(spot) => {
                ui.add(Slider::f64(&mut spot.spot_angle, 0.0..=360.0).text(format!("Spot Angle")));
            }
            Light::DirectionalLight(_direction) => {}
        }
    }

    fn edit_object(object: &mut Object, ui: &mut Ui) {
        if let Some(material) = object.material_mut() {
            let mut whole: i32 = material.refractive_index.floor() as i32;
            let mut frac: Float = material.refractive_index - whole as Float;
            ui.add(Slider::i32(&mut whole, -10..=10).text(format!("Refractive Index whole part")));
            ui.add(
                Slider::f64(&mut frac, -0.0..=0.999)
                    .text(format!("Refractive Index fractional part")),
            );
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

        self.app.recreate_pipeline = selected_changed;
    }

    pub fn edit_cutoff_color(&mut self, ui: &mut Ui) {
        let mut color = self.app.cutoff_color;
        let mut rgb = (color[0] + color[1] + color[2]) / 3.;
        ui.add(Slider::f32(&mut rgb, 0.001..=0.05).text(format!("Cutoff RGB")));
        color[0] = rgb;
        color[1] = rgb;
        color[2] = rgb;
        ui.add(Slider::f32(&mut color[3], 0.001..=0.5).text(format!("Cutoff Alpha")));
        self.app.cutoff_color = color;
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
                if let winit::event::ElementState::Pressed = input.state {
                    match input.virtual_keycode {
                        Some(Key::C) => self.app.clear(),
                        Some(Key::M) => self.app.mode = Mode::DrawMirrorStart,
                        Some(Key::L) => self.app.mode = Mode::DrawPointLight,
                        Some(Key::W) => self.app.mode = Mode::DrawCircleStart,
                        Some(Key::X) => {
                            self.app.refractive_index -= 0.1;
                            self.app.update();
                        }
                        Some(Key::V) => {
                            self.app.refractive_index += 0.1;
                            self.app.update();
                        }
                        Some(Key::U) => {
                            if self.app.max_bounce > 1 {
                                self.app.max_bounce -= 1;
                            }
                        }
                        Some(Key::I) => {
                            if self.app.max_bounce < 100 {
                                self.app.max_bounce += 1;
                            }
                        }

                        _ => {}
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.app.update_mouse_position(nalgebra::Point2::new(
                    (2. * position.x / (sc_desc.width as f64)) - 1.,
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
            }
            _ => {}
        }
    }
}
