use crate::light_garden::*;
use std::slice::Iter;

pub struct Tracer {
    objects: Vec<Object>,
    lights: Vec<Light>,
    has_drawing_object: bool,
    has_drawing_light: bool,
    pub max_bounce: u32,
    pub chunk_size: usize,
    pub cutoff_color: Color,
    pub grid: Grid,
    pub canvas_bounds: Rect,
    pub trace_time_vd: VecDeque<f64>,
    tile_map: TileMap,
    pub debug_key_pressed: bool,
}

impl Tracer {
    pub fn new(canvas_bounds: &Rect) -> Self {
        let light = Light::PointLight(PointLight::new(Point2::new(-0.1, 0.1), 10000, [0.01; 4]));
        let lens = Object::new_lens(P2::new(0.7, 0.), 2., 3.8);
        let mut cubic1 = CubicBezier::new_sample();
        let mut cubic2 = CubicBezier::new_sample2();
        cubic1.scale(0.5, 0.5);
        cubic2.scale(0.5, 0.5);
        let curved_mirror1 = Object::new_curved_mirror(&cubic1);
        let curved_mirror2 = Object::new_curved_mirror(&cubic2);
        let objects = vec![lens, curved_mirror1, curved_mirror2];
        let mut tile_map = TileMap::new(canvas_bounds.width, canvas_bounds.height, 100, 100, 8);
        for obj in objects.iter() {
            tile_map.push_obj(obj);
        }
        println!("canvas_bounds: {:?}", canvas_bounds);
        Tracer {
            lights: vec![light],
            objects,
            has_drawing_object: false,
            has_drawing_light: false,
            max_bounce: 5,
            cutoff_color: [0.001; 4],
            chunk_size: 100,
            grid: Grid::new(canvas_bounds),
            canvas_bounds: *canvas_bounds,
            trace_time_vd: VecDeque::new(),
            tile_map,
            debug_key_pressed: false,
        }
    }

    pub fn clear_objects(&mut self) {
        self.has_drawing_object = false;
        self.objects.clear();
        self.tile_map.clear_tiles();
    }

    pub fn clear(&mut self) {
        self.has_drawing_object = false;
        self.has_drawing_light = false;
        self.objects.clear();
        self.lights.clear();
        self.tile_map.clear_tiles();
    }

    pub fn add_drawing_object(&mut self, obj: Object) {
        if self.has_drawing_object {
            self.tile_map.pop_obj();
            self.objects.pop();
        }
        self.has_drawing_object = true;
        self.tile_map.push_obj(&obj);
        self.objects.push(obj);
    }

    pub fn finish_drawing_object(&mut self, abort: bool) {
        if self.has_drawing_object {
            if abort {
                self.tile_map.pop_obj();
                self.objects.pop();
            }
            self.has_drawing_object = false;
        }
    }

    pub fn add_drawing_light(&mut self, light: Light) {
        if self.has_drawing_light {
            self.lights.pop();
        }
        self.has_drawing_light = true;
        self.lights.push(light);
    }

    pub fn finish_drawing_light(&mut self, abort: bool) {
        if self.has_drawing_light {
            if abort {
                self.lights.pop();
            }
            self.has_drawing_light = false;
        }
    }

    pub fn push_object(&mut self, object: Object) {
        if self.has_drawing_object {
            self.tile_map.pop_obj();
            let dobject = self
                .objects
                .pop()
                .expect("push_object: expected at least one drawing object but found an empty Vec");
            self.tile_map.push_obj(&object);
            self.objects.push(object);
            self.tile_map.push_obj(&dobject);
            self.objects.push(dobject);
        } else {
            self.tile_map.push_obj(&object);
            self.objects.push(object);
        }
    }

    pub fn push_light(&mut self, light: Light) {
        if self.has_drawing_light {
            let dlight = self
                .lights
                .pop()
                .expect("push_light: expected at least one drawing light but found an empty Vec");
            self.lights.push(light);
            self.lights.push(dlight);
        } else {
            self.lights.push(light);
        }
    }

    pub fn index_object(&mut self, ix: usize) -> &mut Object {
        &mut self.objects[ix]
    }

    pub fn index_light(&mut self, ix: usize) -> &mut Light {
        &mut self.lights[ix]
    }

    pub fn replace_object(&mut self, ix: usize, mut object: Object) {
        self.tile_map.update_object(ix, &mut object);
        self.objects[ix] = object;
    }

    pub fn remove_object(&mut self, ix: usize) {
        self.objects.remove(ix);
        self.tile_map.remove_object(ix);
    }

    pub fn remove_light(&mut self, ix: usize) {
        self.lights.remove(ix);
    }

    pub fn object_iterator(&self) -> Iter<'_, Object> {
        self.objects.iter()
    }

    pub fn light_iterator(&self) -> Iter<'_, Light> {
        self.lights.iter()
    }

    pub fn obj_changed(&mut self, obj_index: usize) {
        self.tile_map
            .update_object(obj_index, &mut self.objects[obj_index]);
    }

    pub fn drawing_object_changed(&mut self) {
        if self.has_drawing_object {
            self.obj_changed(self.objects.len() - 1);
        }
    }

    pub fn update_tile_map(&mut self) {
        for (ix, obj) in self.objects.iter_mut().enumerate() {
            self.tile_map.update_object(ix, obj);
        }
    }

    pub fn enable_tile_map(&mut self, enable: bool) {
        if enable {
            self.tile_map.clear_tiles();
            for obj in self.objects.iter_mut() {
                obj.moved = true;
            }
            self.update_tile_map();
        }
        self.tile_map.tile_map_enabled = enable;
    }

    pub fn new_tile_map(&mut self, num_tiles_x: usize, num_tiles_y: usize, num_slabs: usize) {
        let mut tile_map = TileMap::new(
            self.canvas_bounds.width,
            self.canvas_bounds.height,
            num_tiles_x,
            num_tiles_y,
            num_slabs,
        );
        for obj in self.objects.iter() {
            tile_map.push_obj(obj);
        }
        self.tile_map = tile_map;
    }

    pub fn tile_map_enabled(&self) -> bool {
        self.tile_map.tile_map_enabled
    }

    pub fn get_tile_map(&self) -> &TileMap {
        &self.tile_map
    }

    pub fn get_tile(&self, pos: &P2) -> Option<&Tile> {
        self.tile_map.get_tile(pos)
    }

    pub fn resize(&mut self, bounds: &Rect) {
        self.canvas_bounds = *bounds;
        self.grid.update_canvas_bounds(bounds);
        self.tile_map = TileMap::new(bounds.width, bounds.height, 10, 10, 8);
        for obj in self.objects.iter() {
            self.tile_map.push_obj(obj);
        }
    }

    pub fn serialize(&self) -> ron::Result<String> {
        ron::ser::to_string_pretty(
            &(self.objects.clone(), self.lights.clone()),
            ron::ser::PrettyConfig::default(),
        )
    }

    pub fn load(&mut self, data: &str) {
        let (objects, lights) =
            ron::from_str::<(Vec<Object>, Vec<Light>)>(data).expect("Could not load RON file!");
        self.clear();
        self.objects = objects;
        self.lights = lights;
        // update the tile map
        for obj in &self.objects {
            self.tile_map.push_obj(obj);
        }
        // recreate light rays
        for light in self.lights.iter_mut() {
            light.set_num_rays(None);
        }
    }

    pub fn get_trace_time(&self) -> f64 {
        self.trace_time_vd.iter().sum::<f64>() / self.trace_time_vd.len() as f64
    }

    pub fn trace_all_reflective(&mut self) -> Vec<(Vec<P2>, Color)> {
        let mut all_line_strips: Vec<(Vec<P2>, Color)> = Vec::new();
        for light in self.lights.iter() {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let line_strips = light
                    .get_rays()
                    .par_iter()
                    .map(|ray| {
                        let mut line_strip = vec![ray.get_origin()];
                        self.trace_reflective(
                            &mut line_strip,
                            ray,
                            light.get_color(),
                            self.max_bounce,
                        );
                        (line_strip, light.get_color())
                    })
                    .collect::<Vec<(Vec<P2>, Color)>>();
                all_line_strips.extend(line_strips);
            };
            #[cfg(target_arch = "wasm32")]
            {
                let line_strips = light
                    .get_rays()
                    .iter()
                    .map(|ray| {
                        let mut line_strip = vec![ray.get_origin()];
                        self.trace_reflective(
                            &mut line_strip,
                            ray,
                            light.get_color(),
                            self.max_bounce,
                        );
                        (line_strip, light.get_color())
                    })
                    .collect::<Vec<(Vec<P2>, Color)>>();
                all_line_strips.extend(line_strips);
            };
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
        let mut all_lines: Vec<(P2, Color)> = Vec::new();
        for light in self.lights.iter() {
            let mut refractive_index = 1.;
            for obj in self.objects.iter() {
                if obj.contains(&light.get_origin()) {
                    if let Some(material) = obj.material_opt {
                        refractive_index = material.refractive_index;
                    }
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
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
            };
            #[cfg(target_arch = "wasm32")]
            {
                let lines: Vec<(P2, Color)> = light
                    .get_rays()
                    .iter()
                    .map(|ray| {
                        let mut lines = Vec::new();
                        self.trace(
                            &mut lines,
                            ray,
                            light.get_color(),
                            refractive_index,
                            self.max_bounce,
                        );
                        lines
                    })
                    .collect::<Vec<Vec<(P2, Color)>>>()
                    .concat();
                all_lines.extend(lines);
            };
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
            if let ObjectE::CurvedMirror(cm) = obj.object_enum {
                let red = [1., 0., 0., 1.];
                all_lines.push((cm.cubic.points[0], red));
                all_lines.push((cm.cubic.points[1], red));
                all_lines.push((cm.cubic.points[1], red));
                all_lines.push((cm.cubic.points[2], red));
                all_lines.push((cm.cubic.points[2], red));
                all_lines.push((cm.cubic.points[3], red));
            }
        }

        // draw grid
        all_lines.extend(self.grid.get_render_lines());

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
        if self.debug_key_pressed && ray.get_direction().y == -1.0 {
            println!("debug");
        }
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
                let overlaps = match self
                    .tile_map
                    .get_tile(&ray.get_origin())
                    .map(|tile| tile.get_overlaps())
                {
                    Some(o) => o,
                    None => vec![],
                };

                // find the nearest object
                let mut nearest: Float = std::f64::MAX;
                // (intersection point, normal, object index)
                let mut nearest_target: Option<(P2, Normal, usize)> = None;
                if self.tile_map.tile_map_enabled {
                    if let Some(slab) = self.tile_map.index(ray) {
                        for index in slab.object_index_iterator().chain(overlaps.iter()) {
                            if let Some(intersections) =
                                ray.intersect(&self.objects[*index].get_geometry())
                            {
                                for (intersection, normal) in intersections {
                                    let dist_sq =
                                        distance_squared(&ray.get_origin(), &intersection);
                                    if dist_sq < nearest {
                                        nearest = dist_sq;
                                        nearest_target = Some((intersection, normal, *index));
                                    }
                                }
                            }
                        }
                    }
                } else {
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
                }

                if let Some((intersection, normal, index)) = nearest_target {
                    let obj = &self.objects[index];
                    if let Some(material) = obj.material_opt {
                        // get the refracted rays refractive_index
                        let mut refracted_refractive_index = 1.; // air
                        if obj.contains(&ray.get_origin()) {
                            for (ix, o) in self.objects.iter().enumerate() {
                                if ix != index && o.contains(&intersection) {
                                    if let Some(material) = o.get_material() {
                                        refracted_refractive_index = material.refractive_index;
                                        break;
                                    }
                                }
                            }
                        } else {
                            refracted_refractive_index = material.refractive_index;
                        }

                        let result = ray.refract(
                            &intersection,
                            &normal,
                            *refractive_index,
                            refracted_refractive_index,
                        );
                        let (reflected, orefracted, reflectance) = result;
                        rays.push((ray.get_origin(), *color));
                        rays.push((reflected.get_origin(), *color));

                        let refl = reflectance as f32;
                        let omrefl = 1. - refl;
                        let reflected_color =
                            [color[0] * refl, color[1] * refl, color[2] * refl, color[3]];
                        let refracted_color = [
                            color[0] * omrefl,
                            color[1] * omrefl,
                            color[2] * omrefl,
                            color[3],
                        ];
                        back_buffer.push((reflected, reflected_color, *refractive_index));
                        // self.trace(rays, &reflected, color1, refractive_index, max_bounce);
                        if let Some(refracted) = orefracted {
                            back_buffer.push((
                                refracted,
                                refracted_color,
                                refracted_refractive_index,
                            ));
                        }
                    } else {
                        rays.push((ray.get_origin(), *color));
                        rays.push((intersection, *color));
                        back_buffer.push((
                            ray.reflect(&intersection, &normal),
                            *color,
                            *refractive_index,
                        ));
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
