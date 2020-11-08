use crate::light_garden::*;
use collision2d::geo::*;
use imgui::{im_str, ComboBox, Condition, Context, FontSource, ImString, MouseCursor, Slider, Ui};
use std::borrow::Cow;
use std::time::Instant;
use wgpu::{BlendFactor, BlendOperation};

pub struct Gui {
    pub platform: imgui_winit_support::WinitPlatform,
    pub imgui_context: Context,
    last_update_inst: Instant,
    last_cursor: Option<MouseCursor>,
    ixs: [usize; 6],
    demo_open: bool,
}

impl Gui {
    pub fn new(winit_window: &winit::window::Window) -> Self {
        // Set up dear imgui
        let mut imgui_context = Context::create();
        let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui_context);
        platform.attach_window(
            imgui_context.io_mut(),
            winit_window,
            imgui_winit_support::HiDpiMode::Default,
        );
        imgui_context.set_ini_filename(None);

        let font_size = 13.0;
        imgui_context.io_mut().font_global_scale = 1.0;

        imgui_context
            .fonts()
            .add_font(&[FontSource::DefaultFontData {
                config: Some(imgui::FontConfig {
                    oversample_h: 1,
                    pixel_snap_h: true,
                    size_pixels: font_size,
                    ..Default::default()
                }),
            }]);

        #[cfg(not(target_arch = "wasm32"))]
        let last_update_inst = Instant::now();

        Gui {
            platform,
            imgui_context,
            last_update_inst,
            last_cursor: None,
            ixs: [8, 1, 1, 1, 2, 2],
            demo_open: true,
        }
    }

    pub fn gui(&mut self, winit_window: &winit::window::Window, app: &mut LightGarden) -> Ui {
        self.platform
            .prepare_frame(self.imgui_context.io_mut(), &winit_window)
            .expect("Failed to prepare frame");
        let elapsed = self.last_update_inst.elapsed();
        let ctx: &mut imgui::Context = &mut self.imgui_context;
        let mut ixs: &mut [usize; 6] = &mut self.ixs;
        let ui = ctx.frame();
        {
            if app.mode == Mode::NoMode || app.mode == Mode::Selected {
                let window = imgui::Window::new(im_str!("Light Garden"));
                window
                    .size([300.0, 100.0], Condition::FirstUseEver)
                    .build(&ui, || {
                        let mouse_pos = ui.io().mouse_pos;
                        ui.text(im_str!(
                            "Mouse Position: ({:.1},{:.1})",
                            mouse_pos[0],
                            mouse_pos[1]
                        ));

                        if ui.button(im_str!("Add Point Light"), [100., 20.]) {
                            app.mode = Mode::DrawPointLight;
                        }
                        if ui.button(im_str!("Add Rect"), [100., 20.]) {
                            app.mode = Mode::DrawRectStart;
                        }
                        if ui.button(im_str!("Add Circle"), [100., 20.]) {
                            app.mode = Mode::DrawCircleStart;
                        }
                        if ui.button(im_str!("Select"), [100., 20.]) {
                            app.mode = Mode::Selecting(None);
                        }

                        let mut chunk_size = app.chunk_size as u64;
                        Slider::new(im_str!("Rayon Chunk Size"))
                            .range(1..=1000)
                            .build(&ui, &mut chunk_size);
                        app.chunk_size = chunk_size as usize;

                        if let Some(obj) = app.get_selected_object() {
                            Gui::edit_object(obj, &ui);
                        }
                        if app.get_selected_object().is_some() {
                            if ui.button(im_str!("Move Obj"), [100., 20.]) {
                                app.mode = Mode::Move;
                            }
                            if ui.button(im_str!("Rotate"), [100., 20.]) {
                                app.mode = Mode::Rotate;
                            }
                            if ui.button(im_str!("And"), [100., 20.]) {
                                app.mode = Mode::Selecting(Some(LogicOp::And));
                            }
                            if ui.button(im_str!("Or"), [100., 20.]) {
                                app.mode = Mode::Selecting(Some(LogicOp::Or));
                            }
                            if ui.button(im_str!("AndNot"), [100., 20.]) {
                                app.mode = Mode::Selecting(Some(LogicOp::AndNot));
                            }
                        }
                        if let Some(obj_ix) = app.selected_object {
                            ui.text(im_str!("Selected Object Index: {}", obj_ix));
                            if ui.button(im_str!("Delete"), [100., 20.]) {
                                app.delete_selected();
                            }
                        }
                        if let Some(ligh_ix) = app.selected_light {
                            ui.text(im_str!("Selected Light Index: {}", ligh_ix));
                            if ui.button(im_str!("Delete"), [100., 20.]) {
                                app.delete_selected();
                            }
                        }
                        if let Some(light) = app.get_selected_light() {
                            Gui::edit_light(light, &ui);
                            if ui.button(im_str!("Move Light"), [100., 20.]) {
                                app.mode = Mode::Move;
                            }
                        } else {
                            imgui::ColorEdit::new(
                                im_str!("New Light Color"),
                                &mut app.selected_color,
                            )
                            .build(&ui);
                        }

                        Gui::edit_blend(&mut ixs, &ui, app);

                        ui.text(im_str!("Frametime: {:?}", elapsed));
                        ui.text(im_str!("Average Trace Time: {}", app.get_trace_time()));
                    });
            }

            ui.show_demo_window(&mut self.demo_open);
        }
        self.last_update_inst = Instant::now();

        if self.last_cursor != ui.mouse_cursor() {
            self.last_cursor = ui.mouse_cursor();
            self.platform.prepare_render(&ui, winit_window);
        }
        ui
    }

    fn edit_light(light: &mut Light, ui: &Ui) {
        let mut num_rays_mut = light.get_num_rays();
        let num_rays = num_rays_mut;
        Slider::new(im_str!("Num Rays"))
            .range(1..=10000)
            .build(&ui, &mut num_rays_mut);
        if num_rays != num_rays_mut {
            light.set_num_rays(num_rays_mut);
        }
        imgui::ColorEdit::new(im_str!("Light Color"), light.color_mut()).build(&ui);
        match light {
            Light::PointLight(_point) => { /* no user interface elements to add */ }
            Light::SpotLight(spot) => {
                Slider::new(im_str!("Spot Angle"))
                    .range(0.0..=360.0)
                    .build(&ui, &mut spot.spot_angle);
            }
            Light::DirectionalLight(_direction) => {}
        }
    }

    fn edit_object(object: &mut Object, ui: &Ui) {
        if let Some(material) = object.material_mut() {
            let mut whole: i64 = material.refractive_index.floor() as i64;
            let mut frac: Float = material.refractive_index - whole as Float;
            Slider::new(im_str!("Refractive Index whole part"))
                .range(-10..=10)
                .build(&ui, &mut whole);
            Slider::new(im_str!("Refractive Index fractional part:"))
                .range(0.0..=0.999)
                .build(&ui, &mut frac);
            material.refractive_index = whole as Float + frac;
        }
    }

    fn edit_blend(ixs: &mut [usize; 6], ui: &Ui, app: &mut LightGarden) {
        Slider::new(im_str!("Max Bounce:"))
            .range(1..=12)
            .build(&ui, &mut app.max_bounce);
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
        let mut ix = ixs[0];
        let mut ix_changed = false;
        ComboBox::new(im_str!("ColorSrc")).build_simple(
            &ui,
            &mut ixs[0],
            blend_factors,
            &(|i| Cow::from(ImString::new(format!("{:?}", i)))),
        );
        app.color_state_descriptor.color_blend.src_factor = blend_factors[ixs[0]];
        ix_changed |= ix != ixs[0];
        ix = ixs[1];
        ComboBox::new(im_str!("ColorDst")).build_simple(
            &ui,
            &mut ixs[1],
            blend_factors,
            &(|i| Cow::from(ImString::new(format!("{:?}", i)))),
        );
        app.color_state_descriptor.alpha_blend.src_factor = blend_factors[ixs[1]];
        ix_changed |= ix != ixs[1];
        ix = ixs[2];
        ComboBox::new(im_str!("AlphaSrc")).build_simple(
            &ui,
            &mut ixs[2],
            blend_factors,
            &(|i| Cow::from(ImString::new(format!("{:?}", i)))),
        );
        app.color_state_descriptor.color_blend.dst_factor = blend_factors[ixs[2]];
        ix_changed |= ix != ixs[2];
        ix = ixs[3];
        ComboBox::new(im_str!("AlphaDst")).build_simple(
            &ui,
            &mut ixs[3],
            blend_factors,
            &(|i| Cow::from(ImString::new(format!("{:?}", i)))),
        );
        app.color_state_descriptor.alpha_blend.dst_factor = blend_factors[ixs[3]];
        ix_changed |= ix != ixs[3];
        ix = ixs[4];
        let blend_ops: &[BlendOperation] = &[
            BlendOperation::Min,
            BlendOperation::Max,
            BlendOperation::Add,
            BlendOperation::Subtract,
            BlendOperation::ReverseSubtract,
        ];
        ComboBox::new(im_str!("BlendOpColor")).build_simple(
            &ui,
            &mut ixs[4],
            blend_ops,
            &(|i| Cow::from(ImString::new(format!("{:?}", i)))),
        );
        app.color_state_descriptor.color_blend.operation = blend_ops[ixs[4]];
        ix_changed |= ix != ixs[4];
        ix = ixs[5];
        ComboBox::new(im_str!("BlendOpAlpha")).build_simple(
            &ui,
            &mut ixs[5],
            blend_ops,
            &(|i| Cow::from(ImString::new(format!("{:?}", i)))),
        );
        app.color_state_descriptor.alpha_blend.operation = blend_ops[ixs[5]];
        ix_changed |= ix != ixs[5];
        app.recreate_pipeline = ix_changed;
    }
}
