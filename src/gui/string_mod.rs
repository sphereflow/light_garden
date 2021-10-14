use super::*;

impl Gui {
    pub fn string_mod(ui: &mut Ui, string_mod: &mut StringMod) {
        let bnest = string_mod.nested.is_some();
        let mut bcbnest = bnest;
        ui.add(Checkbox::new(&mut bcbnest, "Nested"));
        if bcbnest != bnest {
            string_mod.nested = match bcbnest {
                true => Some(Box::new(StringMod::new())),
                false => None,
            }
        }
        Gui::string_mod_init_curve(string_mod, ui);
        if let Some(nested) = string_mod.nested.as_deref_mut() {
            Window::new("Nested StringMod").show(ui.ctx(), |nested_ui| {
                Gui::string_mod(nested_ui, nested);
            });
        }
        ui.add(DragValue::new::<u64>(&mut string_mod.turns).speed(0.3));
        let mode = &mut string_mod.mode;
        ui.radio_value(mode, StringModMode::Add, "Mode: Add");
        ui.radio_value(mode, StringModMode::Mul, "Mode: Mul");
        ui.radio_value(mode, StringModMode::Pow, "Mode: Pow");
        ui.radio_value(mode, StringModMode::Base, "Mode: Base");
        ui.label("modulo");
        ui.add(
            DragValue::new::<u64>(&mut string_mod.modulo)
                .speed(0.3)
                .clamp_range(0.0..=50000.),
        );
        ui.label("num");
        ui.add(DragValue::new::<u64>(&mut string_mod.num).speed(0.3));
        Gui::edit_string_mod_color(string_mod, ui);
        Gui::string_mod_modulo_colors(string_mod, ui);
    }

    pub fn string_mod_init_curve(string_mod: &mut StringMod, ui: &mut Ui) {
        ui.radio_value(&mut string_mod.init_curve, Curve::Circle, "Circle");
        ui.radio_value(
            &mut string_mod.init_curve,
            Curve::ComplexExp {
                c: Complex::new(0., 1.),
            },
            "Complex",
        );
        ui.radio_value(
            &mut string_mod.init_curve,
            Curve::Hypotrochoid { r: 1, s: 1, d: 1 },
            "Hypotrochoid",
        );
        ui.radio_value(
            &mut string_mod.init_curve,
            Curve::Lissajous {
                a: 1,
                b: 1,
                delta: 0.,
            },
            "Lissajous",
        );
        match string_mod.init_curve {
            Curve::Circle => {}
            Curve::ComplexExp { ref mut c } => {
                let (re, im);
                re = c.re;
                im = c.im;
                let mut angle = (im / re).atan();
                let mut init_len = V2::new(re, im).norm();
                let mut nth_turn = (std::f64::consts::TAU / angle).round() as u64;
                ui.add(
                    Slider::new::<f64>(&mut init_len, 0.9997..=1.0002)
                        .text("Init length")
                        .clamp_to_range(true),
                );
                ui.label("1/nth turn; n:");
                ui.add(DragValue::new::<u64>(&mut nth_turn).clamp_range(4.0..=1000000.0));
                angle = std::f64::consts::TAU / (nth_turn as f64);
                let (im, re) = angle.sin_cos();
                c.re = re * init_len;
                c.im = im * init_len;
            }
            Curve::Hypotrochoid {
                ref mut r,
                ref mut s,
                ref mut d,
            } => {
                ui.label("r:");
                ui.add(DragValue::new::<u64>(r).clamp_range(1.0..=50000.0));
                if s < r {
                    *s = *r;
                }
                ui.label("R:");
                ui.add(DragValue::new::<u64>(s).clamp_range((*r as f32)..=50000.0));
                ui.label("d:");
                ui.add(DragValue::new::<u64>(d).clamp_range(1.0..=50000.0));
            }
            Curve::Lissajous {
                ref mut a,
                ref mut b,
                ref mut delta,
            } => {
                ui.label("a:");
                ui.add(DragValue::new::<u64>(a).clamp_range(1.0..=50000.0));
                ui.label("b:");
                ui.add(DragValue::new::<u64>(b).clamp_range(1.0..=50000.0));
                ui.label("delta:");
                ui.add(DragValue::new::<f64>(delta).clamp_range(1.0..=std::f32::consts::PI));
            }
        }
    }

    pub fn string_mod_modulo_colors(string_mod: &mut StringMod, ui: &mut Ui) {
        if ui.button("Add Color").clicked() {
            string_mod.modulo_colors.push(ModRemColor {
                modulo: 2,
                rem: 0,
                color: [1.; 4],
            });
        }
        if string_mod.modulo_colors.is_empty() {
            return;
        }
        ui.add(
            DragValue::new::<usize>(&mut string_mod.modulo_color_index)
                .clamp_range(0.0..=(string_mod.modulo_colors.len() as f32 - 0.9)),
        );
        let ModRemColor {
            color: c,
            ref mut modulo,
            ref mut rem,
        } = string_mod.modulo_colors[string_mod.modulo_color_index];
        let mut color = Color32::from(Rgba::from_rgba_premultiplied(c[0], c[1], c[2], c[3]));
        egui::widgets::color_picker::color_edit_button_srgba(
            ui,
            &mut color,
            color_picker::Alpha::OnlyBlend,
        );
        ui.add(DragValue::new::<u64>(modulo).clamp_range(1.0..=50000.0));
        ui.add(DragValue::new::<u64>(rem).clamp_range(0.0..=(*modulo - 1) as f32));
        let rgba = Rgba::from(color);
        string_mod.modulo_colors[string_mod.modulo_color_index].color =
            [rgba[0], rgba[1], rgba[2], rgba[3]];
    }

    pub fn string_mod_selector(&mut self, ui: &mut Ui) {
        if ui.button("Screenshot").clicked() {
            self.app.screenshot_path = Some("screenshot.jpg".to_owned());
        }
        ui.add(
            DragValue::new::<usize>(&mut self.app.string_mod_ix)
                .clamp_range(0.0..=self.app.string_mods.len() as f32 - 0.9),
        );
        if ui.button("Add StringMod").clicked() {
            let new = self.get_current_string_mod().clone();
            self.app.string_mods.push(new);
        }
        if self.app.string_mods.len() > 1 && ui.button("Delete current StringMod").clicked() {
            self.app.string_mods.remove(self.app.string_mod_ix);
            if self.app.string_mod_ix >= self.app.string_mods.len() {
                self.app.string_mod_ix -= 1;
            }
        }
    }

    fn edit_string_mod_color(string_mod: &mut StringMod, ui: &mut Ui) {
        let c = string_mod.color;
        let mut color = Color32::from(Rgba::from_rgba_premultiplied(c[0], c[1], c[2], c[3]));
        egui::widgets::color_picker::color_edit_button_srgba(
            ui,
            &mut color,
            color_picker::Alpha::OnlyBlend,
        );
        let rgba = Rgba::from(color);
        string_mod.color = [rgba[0], rgba[1], rgba[2], rgba[3]];
    }

    pub fn get_current_string_mod(&mut self) -> &mut StringMod {
        &mut self.app.string_mods[self.app.string_mod_ix]
    }
}
