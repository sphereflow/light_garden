use super::*;

impl Gui {
    pub fn tile_map(&mut self, ui: &mut Ui) {
        let tm = self.app.tracer.get_tile_map();
        let mut tilesx = tm.get_num_tiles_x();
        let mut tilesy = tm.get_num_tiles_y();
        let mut nslabs = tm.get_num_slabs();
        if ui.add(Slider::new::<usize>(&mut tilesx, 2..=100)).changed()
            || ui.add(Slider::new::<usize>(&mut tilesy, 2..=100)).changed()
            || ui.add(Slider::new::<usize>(&mut nslabs, 4..=32)).changed()
        {
            self.app.tracer.new_tile_map(tilesx, tilesy, nslabs);
        }
    }

    pub fn select_tile(&self, ui: &mut Ui) {
        if let Some(tile) = self.app.tracer.get_tile(&self.app.get_mouse_pos()) {
            ui.label(format!("{}", tile));
        }
    }

    pub fn tile_selected(&self, ui: &mut Ui, tile: &Tile) {
        let slab = tile.index(&Unit::new_normalize(
            self.app.get_mouse_pos() - tile.aabb.get_origin(),
        ));
        ui.label(format!("{}", slab));
    }

    pub fn toggle_tile_map(&mut self, ui: &mut Ui) {
        let mut tile_map_enabled = self.app.tracer.tile_map_enabled();
        if ui
            .add(Checkbox::new(&mut tile_map_enabled, "TileMap enabled"))
            .changed()
        {
            self.app.tracer.enable_tile_map(tile_map_enabled);
        }
    }
}
