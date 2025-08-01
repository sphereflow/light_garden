use super::*;

impl Gui {
    pub fn settings(&mut self, ui: &mut Ui) {
        let mut chunk_size = self.app.tracer.chunk_size as u32;
        ui.add(Slider::new::<u32>(&mut chunk_size, 1..=1000).text("Rayon Chunk Size"));
        self.app.tracer.chunk_size = chunk_size as usize;

        self.edit_blend(ui);

        self.edit_cutoff_color(ui);

        self.toggle_render_to_texture(ui);

        self.toggle_tile_map(ui);
    }

    pub fn edit_light(light: &mut Light, ui: &mut Ui) {
        let mut update_light = false;
        match light {
            Light::PointLight(_point) => { /* no user interface elements to add */ }
            Light::SpotLight(spot) => {
                // spot angle
                // conversion radian -> degrees
                let mut spot_angle = spot.spot_angle * 180. / PI;
                let old_spot_angle = spot_angle;
                ui.add(Slider::new::<f64>(&mut spot_angle, 0.0..=360.0).text("Spot Angle"));
                if (spot_angle - old_spot_angle).abs() > Float::EPSILON {
                    // conversion degrees -> radian
                    spot.spot_angle = spot_angle * PI / 180.;
                    update_light = true;
                }
            }
            Light::DirectionalLight(_direction) => {}
        }

        // num rays
        let mut num_rays_mut = light.get_num_rays();
        let num_rays = num_rays_mut;
        ui.add(Slider::new::<usize>(&mut num_rays_mut, 1..=30000).text("Num Rays"));
        if num_rays != num_rays_mut || update_light {
            light.set_num_rays(Some(num_rays_mut));
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

    fn edit_blend(&mut self, ui: &mut Ui) {
        ui.add(Slider::new::<u32>(&mut self.app.tracer.max_bounce, 1..=20).text("Max Bounce:"));
        let blend_factors: &[BlendFactor] = &[
            BlendFactor::Zero,
            BlendFactor::One,
            BlendFactor::Src,
            BlendFactor::OneMinusSrc,
            BlendFactor::SrcAlpha,
            BlendFactor::OneMinusSrcAlpha,
            BlendFactor::Dst,
            BlendFactor::OneMinusDst,
            BlendFactor::DstAlpha,
            BlendFactor::OneMinusDstAlpha,
            BlendFactor::SrcAlphaSaturated,
            BlendFactor::Constant,
            BlendFactor::OneMinusConstant,
        ];
        let mut selected_changed = false;

        if let Some(blend_state) = self.app.color_state_descriptor.blend.as_mut() {
            let combo = |(text, var): (&str, &mut BlendFactor), ui: &mut Ui| {
                ComboBox::from_label(text)
                    .selected_text(format!("{var:?}"))
                    .show_ui(ui, |ui| {
                        blend_factors.iter().for_each(|bf| {
                            ui.selectable_value(var, *bf, format!("{bf:?}"));
                        });
                    })
                    .response
                    .changed()
            };
            let old_blend = *blend_state;

            selected_changed = [
                combo(("ColorSrc", &mut blend_state.color.src_factor), ui),
                combo(("ColorDst", &mut blend_state.color.dst_factor), ui),
                combo(("AlphaSrc", &mut blend_state.alpha.src_factor), ui),
                combo(("AlphaDst", &mut blend_state.alpha.dst_factor), ui),
            ]
            .iter()
            .any(|b| *b);

            let blend_ops: &[BlendOperation] = &[
                BlendOperation::Min,
                BlendOperation::Max,
                BlendOperation::Add,
                BlendOperation::Subtract,
                BlendOperation::ReverseSubtract,
            ];

            let combo = |(text, var): (&str, &mut BlendOperation), ui: &mut Ui| {
                let res = ComboBox::from_label(text)
                    .selected_text(format!("{var:?}"))
                    .show_ui(ui, |ui| {
                        blend_ops.iter().for_each(|bo| {
                            ui.selectable_value(var, *bo, format!("{bo:?}"));
                        });
                    })
                    .response
                    .changed();
                if res {
                    println!("combo changed");
                }
                res
            };

            // ComboBox does not emit the changed signal seems to be a bug in egui
            // selected_changed |= combo(("BlendOpColor", &mut blend_state.color.operation), ui);
            // selected_changed |= combo(("BlendOpAlpha", &mut blend_state.alpha.operation), ui);
            // workaround =>
            let _ = combo(("BlendOpColor", &mut blend_state.color.operation), ui);
            let _ = combo(("BlendOpAlpha", &mut blend_state.alpha.operation), ui);
            selected_changed = old_blend != *blend_state;
        }

        self.app.recreate_pipelines |= selected_changed;
    }

    pub fn edit_cutoff_color(&mut self, ui: &mut Ui) {
        let mut color = self.app.tracer.cutoff_color;
        let mut rgb = (color[0] + color[1] + color[2]) / 3.;
        ui.add(Slider::new::<f32>(&mut rgb, 0.00001..=0.05).text("Cutoff RGB"));
        color[0] = rgb;
        color[1] = rgb;
        color[2] = rgb;
        ui.add(Slider::new::<f32>(&mut color[3], 0.00001..=0.05).text("Cutoff Alpha"));
        self.app.tracer.cutoff_color = color;
    }

    pub fn toggle_render_to_texture(&mut self, ui: &mut Ui) {
        let mut render_to_texture = self.app.get_render_to_texture();
        ui.add(Checkbox::new(&mut render_to_texture, "render to texture"));
        self.app.set_render_to_texture(render_to_texture);
    }
}
