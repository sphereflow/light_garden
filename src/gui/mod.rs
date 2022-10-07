use crate::light_garden::*;
use collision2d::geo::*;
use egui::*;
use instant::Instant;
use na::Complex;
use std::f64::consts::PI;
use wgpu::{BlendFactor, BlendOperation};

#[cfg(not(target_arch = "wasm32"))]
use rfd::FileDialog;

mod grid;
mod input;
mod settings;
mod string_mod;
mod tile_map;

pub struct Gui {
    pub winit_state: egui_winit::State,
    pub scale_factor: f32,
    last_update_inst: Instant,
    last_cursor: Option<Pos2>,
    gui_contains_pointer: bool,
    pub app: LightGarden,
    pub ui_mode: UiMode,
}

impl Gui {
    pub fn name(&self) -> &str {
        "Light Garden"
    }

    pub fn update(&mut self, ctx: &Context, winit_window: &winit::window::Window) -> FullOutput {
        let input = self.winit_state.take_egui_input(winit_window);
        ctx.begin_frame(input);
        let bdisplay_ui = matches!(
            self.app.mode,
            Mode::Selected
                | Mode::Selecting(None)
                | Mode::DrawConvexPolygon { .. }
                | Mode::StringMod
                | Mode::SelectTile
                | Mode::TileSelected { .. }
        );
        if !bdisplay_ui {
            self.gui_contains_pointer = false;
        }
        if bdisplay_ui {
            let window = Window::new("Light Garden");
            window
                .default_size(Vec2::new(300.0, 100.0))
                .show(ctx, |ui| {
                    self.last_cursor = ui.input().pointer.interact_pos();
                    if let Some(mouse_pos) = self.last_cursor {
                        ui.label(format!(
                            "Mouse Position: ({:.1},{:.1})",
                            mouse_pos.x, mouse_pos.y
                        ));
                    }

                    self.display_mode(ui);

                    match &self.app.mode {
                        Mode::StringMod => {
                            self.ui_mode = UiMode::StringMod;
                        }
                        Mode::Selected | Mode::EditObject => {
                            self.ui_mode = UiMode::Selected;
                        }
                        Mode::DrawConvexPolygon { .. } => {
                            self.draw_convex_polygon(ui);
                        }
                        Mode::SelectTile => {
                            self.select_tile(ui);
                        }
                        Mode::TileSelected { tile } => {
                            self.tile_selected(ui, tile);
                        }
                        _ => {}
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
                        UiMode::TileMap => {
                            self.tile_map(ui);
                        }
                        UiMode::StringMod => {
                            self.string_mod_selector(ui);
                            Gui::string_mod(ui, self.get_current_string_mod());
                        }
                        UiMode::Exiting => {}
                    }

                    let elapsed = self.last_update_inst.elapsed();
                    ui.label(format!("Frametime: {:.2?}", elapsed));
                    ui.label(format!(
                        "Average Trace Time: {:.2}",
                        self.app.tracer.get_trace_time()
                    ));
                    self.gui_contains_pointer = ctx.is_pointer_over_area();
                });
        }

        self.last_update_inst = Instant::now();
        ctx.end_frame()
    }
}

impl Gui {
    pub fn new(
        winit_window: &winit::window::Window,
        event_loop: &winit::event_loop::EventLoop<()>,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        let size = winit_window.inner_size();
        let last_update_inst = Instant::now();
        let app = LightGarden::new(
            collision2d::geo::Rect::from_tlbr(1., -1., -1., 1.),
            surface_config.format,
        );
        let winit_state = egui_winit::State::new(&event_loop);

        Gui {
            winit_state,
            scale_factor: winit_window.scale_factor() as f32,
            last_update_inst,
            last_cursor: None,
            gui_contains_pointer: false,
            app,
            ui_mode: UiMode::new(),
        }
    }

    fn main(&mut self, ui: &mut Ui) {
        if ui.button("(A)dd ...").clicked() {
            self.ui_mode = UiMode::Add;
        }
        if ui.button("S(e)ttings").clicked() {
            self.ui_mode = UiMode::Settings;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if ui.button("Screenshot").clicked() {
                if let Some(path_buf) = FileDialog::new()
                    .set_file_name("Screenshot.jpg")
                    .save_file()
                {
                    if let Some(path) = path_buf.to_str() {
                        self.app.screenshot_path = Some(path.to_string());
                    }
                }
            }
        }
        if ui.button("(G)rid").clicked() {
            self.ui_mode = UiMode::Grid;
        }
        if ui.button("(T)ile Map").clicked() {
            self.ui_mode = UiMode::TileMap;
            self.app.mode = Mode::SelectTile;
        }
        if ui.button("St(r)ing mod").clicked() {
            self.ui_mode = UiMode::StringMod;
            self.app.mode = Mode::StringMod;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.save_file(ui);
            self.load_file(ui);
        }
        if ui.button("(Q)it").clicked() {
            self.ui_mode = UiMode::Exiting;
        }
    }

    fn display_mode(&mut self, ui: &mut Ui) {
        ui.label(format!("App mode: {}", self.app.mode));
        ui.label(format!("Ui mode: {:?}", self.ui_mode));
    }

    fn selected(&mut self, ui: &mut Ui) {
        if let Some(obj) = self.app.get_selected_object() {
            Gui::edit_object(obj, ui);
        }

        if ui.button("(E)dit").clicked() {
            self.app.mode = Mode::EditObject;
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
            if ui.button("Add Con(v)ex Polygon").clicked() {
                self.app.mode = Mode::DrawConvexPolygon { points: Vec::new() };
            }
            if ui.button("Add C(u)rved Mirror").clicked() {
                self.app.mode = Mode::DrawCurvedMirror { points: Vec::new() };
            }
        }
    }

    fn edit(&mut self, ui: &mut Ui) {
        if self.app.get_selected_object().is_some() {
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

    fn draw_convex_polygon(&mut self, ui: &mut Ui) {
        if ui.button("Finish").clicked() {
            self.app.mode = Mode::Selecting(None);
            self.ui_mode = UiMode::Add;
            self.app.tracer.finish_drawing_object(false);
        }
    }

    fn edit_object(object: &mut Object, ui: &mut Ui) {
        if let Some(material) = object.material_mut() {
            let mut whole: i32 = material.refractive_index.floor() as i32;
            let mut frac: Float = material.refractive_index - whole as Float;
            ui.add(Slider::new::<i32>(&mut whole, -10..=10).text("Refractive Index whole part"));
            ui.add(
                Slider::new::<f64>(&mut frac, -0.0..=0.999)
                    .text("Refractive Index fractional part"),
            );
            material.refractive_index = whole as Float + frac;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_file(&mut self, ui: &mut Ui) {
        use std::thread::spawn;

        if ui.button("Save ...").clicked() {
            let arc = self.app.save_file_path.clone();
            spawn(move || {
                let mut app_path = arc.lock().expect("gui::load_file: failed to aquire mutex");
                if let Some(path_buf) = FileDialog::new().set_file_name("save.ron").save_file() {
                    *app_path = path_buf.to_str().map(|s| s.into());
                }
            });
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_file(&mut self, ui: &mut Ui) {
        if ui.button("Load ...").clicked() {
            let arc = self.app.load_file_path.clone();
            std::thread::spawn(move || {
                let mut app_path = arc.lock().expect("gui::load_file: failed to aquire mutex");
                if let Some(path_buf) = FileDialog::new().pick_file() {
                    *app_path = path_buf.to_str().map(|s| s.into());
                }
            });
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
    StringMod,
    TileMap,
    Exiting,
}

impl UiMode {
    pub fn new() -> UiMode {
        UiMode::Main
    }
}

impl Default for UiMode {
    fn default() -> Self {
        Self::new()
    }
}
