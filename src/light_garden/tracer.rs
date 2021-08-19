use crate::light_garden::*;

pub struct Tracer {
    pub objects: Vec<Object>,
    pub lights: Vec<Light>,
    pub drawing_object: Option<Object>,
    pub drawing_light: Option<Light>,
    pub max_bounce: u32,
    pub chunk_size: usize,
    pub cutoff_color: Color,
    pub grid: Grid,
    pub canvas_bounds: Rect,
    pub trace_time_vd: VecDeque<f64>,
    tile_map: TileMap,
}

impl Tracer {
    pub fn new(canvas_bounds: &Rect) -> Self {
        let light = Light::PointLight(PointLight::new(Point2::new(-0.1, 0.1), 10000, [0.01; 4]));
        let lens = Object::new_lens(P2::new(0.7, 0.), 2., 3.8);
        let mut cubic1 = CubicBezier::new_sample();
        cubic1.scale(0.5, 0.5);
        let curved_mirror1 = Object::CurvedMirror(CurvedMirror { cubic: cubic1 });
        let mut cubic2 = CubicBezier::new_sample2();
        cubic2.scale(0.5, 0.5);
        let curved_mirror2 = Object::CurvedMirror(CurvedMirror { cubic: cubic2 });
        let objects = vec![lens, curved_mirror1, curved_mirror2];
        let mut tile_map = TileMap::new(canvas_bounds.width, canvas_bounds.height, 100, 100, 8);
        for (ix, obj) in objects.iter().enumerate() {
            tile_map.add_obj(ix, obj);
        }
        println!("canvas_bounds: {:?}", canvas_bounds);
        for tile in &tile_map.tiles {
            println!("aabb: {:?}", tile.aabb);
        }
        Tracer {
            lights: vec![light],
            objects,
            drawing_object: None,
            drawing_light: None,
            max_bounce: 5,
            cutoff_color: [0.001; 4],
            chunk_size: 100,
            grid: Grid::new(&canvas_bounds),
            canvas_bounds: *canvas_bounds,
            trace_time_vd: VecDeque::new(),
            tile_map,
        }
    }

    pub fn add_drawing_object(&mut self) {
        if let Some(obj) = self.drawing_object.take() {
            self.objects.push(obj);
        }
    }

    pub fn add_drawing_light(&mut self) {
        if let Some(light) = self.drawing_light.take() {
            self.lights.push(light);
        }
    }

    pub fn obj_changed(&mut self, obj_index: usize) {
        self.tile_map.delete_obj(obj_index);
        self.tile_map.add_obj(obj_index, &self.objects[obj_index]);
    }

    pub fn resize(&mut self, bounds: &Rect) {
        self.canvas_bounds = *bounds;
        self.grid.update_canvas_bounds(bounds);
        self.tile_map = TileMap::new(bounds.width, bounds.height, 100, 100, 20);
        for (ix, obj) in self.objects.iter().enumerate() {
            self.tile_map.add_obj(ix, obj);
        }
    }

    pub fn get_trace_time(&self) -> f64 {
        self.trace_time_vd.iter().sum::<f64>() / self.trace_time_vd.len() as f64
    }

    pub fn trace_all_reflective(&mut self) -> Vec<(Vec<P2>, Color)> {
        if let Some(dro) = self.drawing_object.clone() {
            self.objects.push(dro);
        }
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
        if self.drawing_object.is_some() {
            self.objects.pop();
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
        if let Some(dro) = self.drawing_object.as_ref() {
            self.objects.push(dro.clone());
        }
        if let Some(drl) = self.drawing_light.as_ref() {
            self.lights.push(drl.clone());
        }
        let mut all_lines: Vec<(P2, Color)> = Vec::new();
        for light in self.lights.iter() {
            let mut refractive_index = 1.;
            for obj in self.objects.iter() {
                if let Object::Circle(c, material) = obj {
                    if c.contains(&light.get_origin()) {
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
        if self.drawing_object.is_some() {
            self.objects.pop();
        }
        if self.drawing_light.is_some() {
            self.lights.pop();
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
            if let Object::CurvedMirror(cm) = obj {
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

                // find the nearest object
                let mut nearest: Float = std::f64::MAX;
                // (intersection point, normal, object index)
                let mut nearest_target: Option<(P2, Normal, usize)> = None;
                if let Some(slab) = self.tile_map.index(ray) {
                    for index in &slab.obj_indices {
                        if let Some(intersections) =
                            ray.intersect(&self.objects[*index].get_geometry())
                        {
                            for (intersection, normal) in intersections {
                                let dist_sq = distance_squared(&ray.get_origin(), &intersection);
                                if dist_sq < nearest {
                                    nearest = dist_sq;
                                    nearest_target = Some((intersection, normal, *index));
                                }
                            }
                        }
                    }
                }

                if let Some((intersection, normal, index)) = nearest_target {
                    let obj = &self.objects[index];
                    match obj {
                        Object::StraightMirror(_) | Object::CurvedMirror(_) => {
                            rays.push((ray.get_origin(), *color));
                            rays.push((intersection, *color));
                            back_buffer.push((
                                ray.reflect(&intersection, &normal),
                                *color,
                                *refractive_index,
                            ));
                        }

                        Object::Rect(_, material)
                        | Object::Circle(_, material)
                        | Object::Lens(_, material)
                        | Object::ConvexPolygon(_, material)
                        | Object::Geo(_, material) => {
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
                        }
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
