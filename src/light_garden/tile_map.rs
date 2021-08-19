use crate::light_garden::*;

// tiles go from the top left in rows to the bottom right
#[derive(Clone, PartialEq, Debug)]
pub struct TileMap {
    window_width: Float,
    window_height: Float,
    num_tilesx: usize,
    num_tilesy: usize,
    num_slabs: usize,
    pub tiles: Vec<Tile>,
}

impl TileMap {
    pub fn new(
        window_width: Float,
        window_height: Float,
        num_tilesx: usize,
        num_tilesy: usize,
        num_slabs: usize,
    ) -> Self {
        let mut directions = Vec::new();
        for i in 0..num_slabs {
            let radian_angle = i as Float * TAU / (num_slabs as Float);
            let (sine, cosine) = radian_angle.sin_cos();
            directions.push(V2::new(sine, cosine));
        }
        directions.push(V2::new(0., 1.));
        let mut tiles = Vec::new();
        for ixy in 0..num_tilesy {
            for ixx in 0..num_tilesx {
                let mut slabs = Vec::new();
                let aabb = TileMap::get_aabb(
                    window_width,
                    window_height,
                    num_tilesx,
                    num_tilesy,
                    ixx,
                    ixy,
                );
                for left_right in directions.windows(2) {
                    slabs.push(Slab::new(&aabb, &left_right[0], &left_right[1]));
                }
                tiles.push(Tile { aabb, slabs });
            }
        }
        for slab in &tiles[4].slabs {
            println!("directionl: {:?}", slab.rleft.get_direction());
            println!("directionr: {:?}", slab.rright.get_direction());
        }
        TileMap {
            window_width,
            window_height,
            num_tilesx,
            num_tilesy,
            num_slabs,
            tiles,
        }
    }
    pub fn get_aabb(
        window_width: Float,
        window_height: Float,
        num_tilesx: usize,
        num_tilesy: usize,
        ixx: usize,
        ixy: usize,
    ) -> Aabb {
        let stepx = window_width / num_tilesx as Float;
        let stepy = window_height / num_tilesy as Float;
        let left = ixx as Float * stepx - window_width * 0.5;
        let bottom = ixy as Float * stepy - window_height * 0.5;
        Aabb::from_tlbr(bottom + stepy, left, bottom, left + stepx)
    }

    pub fn add_obj(&mut self, ix: usize, obj: &Object) {
        for tile in self.tiles.iter_mut() {
            for slab in tile.slabs.iter_mut() {
                if slab.overlaps(obj) {
                    slab.obj_indices.push(ix);
                }
            }
        }
    }
    pub fn delete_obj(&mut self, ix: usize) {
        for tile in self.tiles.iter_mut() {
            for slab in tile.slabs.iter_mut() {
                for i in (0..slab.obj_indices.len()).rev() {
                    let index = slab.obj_indices[i];
                    if index == ix {
                        slab.obj_indices.remove(i);
                    }
                }
            }
        }
    }
    pub fn index(&self, ray: &Ray) -> Option<&Slab> {
        self.get_tile(ray.get_origin())
            .map(|tile| tile.index(ray.get_direction()))
    }
    pub fn get_tile(&self, pos: P2) -> Option<&Tile> {
        let mut pos = pos;
        pos += V2::new(self.window_width * 0.5, self.window_height * 0.5);
        let stepx = self.window_width / self.num_tilesx as Float;
        let stepy = self.window_height / self.num_tilesy as Float;
        let ixx = (pos.x / stepx) as usize;
        let ixy = (pos.y / stepy) as usize;
        if ixx < self.num_tilesx && ixy < self.num_tilesy {
            Some(&self.tiles[ixx + ixy * self.num_tilesx])
        } else {
            None
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Tile {
    pub aabb: Aabb,
    pub slabs: Vec<Slab>,
}

impl Tile {
    pub fn new(aabb: Aabb, slabs: &[Slab]) -> Self {
        Tile {
            aabb,
            slabs: slabs.into(),
        }
    }
    pub fn index(&self, direction: U2) -> &Slab {
        let mut radian_angle = direction.y.acos();
        if direction.x < 0.0 {
            radian_angle = TAU - radian_angle;
        }
        let ix = ((self.slabs.len() as Float * radian_angle / TAU) - EPSILON) as usize;
        &self.slabs[ix]
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Slab {
    rleft: Ray,
    rright: Ray,
    ls: LineSegment,
    pub obj_indices: Vec<usize>,
}

impl Slab {
    pub fn new(aabb: &Aabb, direction_left: &V2, direction_right: &V2) -> Self {
        let qleft = get_quadrant(direction_left);
        let qright = get_quadrant(direction_right);
        let rleft = qleft.get_left_ray(aabb, direction_left);
        let rright = qright.get_right_ray(aabb, direction_right);
        Slab {
            rleft,
            rright,
            ls: LineSegment::from_ab(rleft.get_origin(), rright.get_origin()),
            obj_indices: Vec::new(),
        }
    }
    pub fn overlaps(&self, obj: &Object) -> bool {
        let geo = obj.get_geometry();
        between_rays(&obj.get_origin(), &self.rleft, &self.rright)
            || self.rleft.intersect(&geo).is_some()
            || self.rright.intersect(&geo).is_some()
            || geo.intersect(&self.ls).is_some()
    }
}

fn between_rays(p: &P2, left: &Ray, right: &Ray) -> bool {
    is_clockwise_directions(&left.get_direction(), &(p - left.get_origin()))
        && is_clockwise_directions(&(p - right.get_origin()), &right.get_direction())
}

//            |
//            |
//     Q3     |     Q0
//            |
//            |
//------------+------------
//            |
//            |
//     Q2     |     Q1
//            |
//            |
#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum Quadrant {
    Q0,
    Q1,
    Q2,
    Q3,
}

fn get_quadrant(direction: &V2) -> Quadrant {
    if direction.x >= 0.0 && direction.y >= 0.0 {
        Quadrant::Q0
    } else if direction.x >= 0.0 && direction.y < 0.0 {
        Quadrant::Q1
    } else if direction.x < 0.0 && direction.y < 0.0 {
        Quadrant::Q2
    } else {
        Quadrant::Q3
    }
}

impl Quadrant {
    fn get_left_ray(&self, aabb: &Aabb, direction_left: &V2) -> Ray {
        let mut left = Ray::from_origin(self.get_point_left(aabb), *direction_left);
        let dist = Line::new(left.get_origin(), left.get_normal().into_inner())
            .distance(&self.get_opposing_point(aabb));
        left.shift(-dist.abs() * left.get_direction().into_inner());
        left
    }

    fn get_right_ray(&self, aabb: &Aabb, direction_right: &V2) -> Ray {
        let mut right = Ray::from_origin(self.get_point_right(aabb), *direction_right);
        let dist = Line::new(right.get_origin(), right.get_normal().into_inner())
            .distance(&self.get_opposing_point(aabb));
        right.shift(-dist.abs() * right.get_direction().into_inner());
        right
    }

    fn get_point_left(&self, aabb: &Aabb) -> P2 {
        match self {
            Quadrant::Q0 => P2::new(aabb.get_left(), aabb.get_top()),
            Quadrant::Q1 => P2::new(aabb.get_right(), aabb.get_top()),
            Quadrant::Q2 => P2::new(aabb.get_right(), aabb.get_bottom()),
            Quadrant::Q3 => P2::new(aabb.get_left(), aabb.get_bottom()),
        }
    }

    fn get_point_right(&self, aabb: &Aabb) -> P2 {
        match self {
            Quadrant::Q0 => P2::new(aabb.get_right(), aabb.get_bottom()),
            Quadrant::Q1 => P2::new(aabb.get_left(), aabb.get_bottom()),
            Quadrant::Q2 => P2::new(aabb.get_left(), aabb.get_top()),
            Quadrant::Q3 => P2::new(aabb.get_right(), aabb.get_top()),
        }
    }

    fn get_opposing_point(&self, aabb: &Aabb) -> P2 {
        let l = aabb.get_left();
        let r = aabb.get_right();
        let t = aabb.get_top();
        let b = aabb.get_bottom();
        match self {
            Quadrant::Q0 => P2::new(l, b),
            Quadrant::Q1 => P2::new(l, t),
            Quadrant::Q2 => P2::new(r, t),
            Quadrant::Q3 => P2::new(r, b),
        }
    }
}
