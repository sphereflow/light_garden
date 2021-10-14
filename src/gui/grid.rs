use super::*;

impl Gui {
    pub fn grid(&mut self, ui: &mut Ui) {
        // toggle snap to grid
        ui.add(Checkbox::new(&mut self.app.tracer.grid.show, "Show grid"));
        ui.label("Snap to grid while pressing the left shift key");
        self.grid_size(ui);
        self.edit_grid_color(ui);
    }

    pub fn grid_size(&mut self, ui: &mut Ui) {
        let mut grid_size = self.app.tracer.grid.get_dist();
        if ui
            .add(Slider::new::<f64>(&mut grid_size, 0.01..=0.1).text("Grid size"))
            .changed()
        {
            self.app
                .tracer
                .grid
                .set_dist(grid_size, &self.app.get_canvas_bounds());
        }
    }

    fn edit_grid_color(&mut self, ui: &mut Ui) {
        let c = self.app.tracer.grid.get_color();
        let mut color = Color32::from(Rgba::from_rgba_premultiplied(c[0], c[1], c[2], c[3]));
        egui::widgets::color_picker::color_edit_button_srgba(
            ui,
            &mut color,
            color_picker::Alpha::OnlyBlend,
        );
        let rgba = Rgba::from(color);
        self.app
            .tracer
            .grid
            .set_color([rgba[0], rgba[1], rgba[2], rgba[3]]);
    }
}
