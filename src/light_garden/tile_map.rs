use crate::light_garden::*;
use std::fmt::{Display, Formatter, Result};
use std::slice::Iter;

// tiles go from the top left in rows to the bottom right
#[derive(Clone, PartialEq, Debug)]
pub struct TileMap {
    window_width: Float,
    window_height: Float,
    num_tilesx: usize,
    num_tilesy: usize,
    num_slabs: usize,
    pub tiles: Vec<Tile>,
    pub tile_map_enabled: bool,
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
                tiles.push(Tile {
                    aabb,
                    slabs,
                    range_map: Vec::new(),
                });
            }
        }
        TileMap {
            window_width,
            window_height,
            num_tilesx,
            num_tilesy,
            num_slabs,
            tiles,
            tile_map_enabled: true,
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

    pub fn get_num_tiles_x(&self) -> usize {
        self.num_tilesx
    }

    pub fn get_num_tiles_y(&self) -> usize {
        self.num_tilesy
    }

    pub fn get_num_slabs(&self) -> usize {
        self.num_slabs
    }

    pub fn push_obj(&mut self, obj: &Object) {
        if self.tile_map_enabled {
            for tile in self.tiles.iter_mut() {
                tile.push_overlap(obj);
            }
        }
    }

    pub fn pop_obj(&mut self) {
        if self.tile_map_enabled {
            for tile in self.tiles.iter_mut() {
                tile.pop_overlap();
            }
        }
    }

    pub fn remove_object(&mut self, ix: usize) {
        if self.tile_map_enabled {
            for tile in self.tiles.iter_mut() {
                tile.remove_overlap(ix);
            }
        }
    }

    pub fn update_object(&mut self, ix: usize, object: &mut Object) {
        if object.moved && self.tile_map_enabled {
            object.moved = false;
            for tile in self.tiles.iter_mut() {
                tile.update_overlap(ix, object);
            }
        }
    }

    pub fn index(&self, ray: &Ray) -> Option<&Slab> {
        if self.tile_map_enabled {
            self.get_tile(&ray.get_origin())
                .map(|tile| tile.index(&ray.get_direction()))
        } else {
            None
        }
    }

    pub fn get_tile(&self, pos: &P2) -> Option<&Tile> {
        let mut pos = *pos;
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

    pub fn clear_tiles(&mut self) {
        if self.tile_map_enabled {
            for tile in self.tiles.iter_mut() {
                tile.clear();
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct SlabRange {
    pub start: usize,
    pub end: usize,
    num_slabs: usize,
}

impl IntoIterator for SlabRange {
    type Item = usize;
    type IntoIter = SlabRangeIterator;
    fn into_iter(self) -> Self::IntoIter {
        SlabRangeIterator {
            range: self,
            done: false,
        }
    }
}

pub struct SlabRangeIterator {
    range: SlabRange,
    done: bool,
}

impl Iterator for SlabRangeIterator {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            None
        } else {
            let res = Some(self.range.start);
            if self.range.start == self.range.end {
                self.done = true;
            }
            self.range.start = (self.range.start + 1) % self.range.num_slabs;
            res
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Tile {
    pub aabb: Aabb,
    pub slabs: Vec<Slab>,
    pub range_map: Vec<Option<SlabRange>>,
}

impl Tile {
    pub fn new(aabb: Aabb, slabs: &[Slab]) -> Self {
        Tile {
            aabb,
            slabs: slabs.into(),
            range_map: Vec::new(),
        }
    }

    pub fn index(&self, direction: &U2) -> &Slab {
        &self.slabs[self.get_index(direction)]
    }

    pub fn push_overlap(&mut self, obj: &Object) {
        self.range_map.push(None);
        self.update_overlap(self.range_map.len() - 1, obj);
    }

    pub fn pop_overlap(&mut self) {
        self.remove_object_from_slabs(self.range_map.len() - 1, false);
    }

    pub fn remove_overlap(&mut self, ix: usize) {
        self.remove_object_from_slabs(ix, false);
    }

    fn get_index(&self, direction: &U2) -> usize {
        let mut radian_angle = direction.y.acos();
        if direction.x < 0.0 {
            radian_angle = TAU - radian_angle;
        }
        ((self.slabs.len() as Float * radian_angle / TAU) - EPSILON) as usize
    }

    pub fn update_overlap(&mut self, obj_index: usize, obj: &Object) -> bool {
        let obj_aabb = obj.get_aabb();
        let res = obj_aabb.intersect(&self.aabb).is_some()
            || obj_aabb.contains(&self.aabb.get_origin())
            || self.aabb.contains(&obj_aabb.get_origin());
        if res {
            // the object overlaps self.aabb
            self.remove_object_from_slabs(obj_index, true);
            self.range_map[obj_index] = None;
        } else {
            // object is outside of self.aabb
            self.update_object(obj_index, obj);
        }
        res
    }

    /// this function calculates the objects ```SlabRange``` and puts it
    /// into the tiles range map.
    /// this function should only be called by update_overlap
    fn update_object(&mut self, obj_index: usize, obj: &Object) {
        self.range_map[obj_index] = self.get_range(&obj.get_aabb());
        if let Some(srange) = self.range_map[obj_index] {
            for ix in srange.into_iter() {
                self.slabs[ix].insert_object(obj_index);
            }
        }
    }

    fn remove_object_from_slabs(&mut self, obj_index: usize, keep_indices: bool) {
        for slab in self.slabs.iter_mut() {
            slab.remove_object(obj_index, keep_indices);
        }
        if !keep_indices {
            self.range_map.remove(obj_index);
        }
    }

    pub fn clear(&mut self) {
        self.range_map.clear();
        for slab in self.slabs.iter_mut() {
            slab.clear();
        }
    }

    pub fn get_overlaps(&self) -> Vec<usize> {
        let mut res = Vec::new();
        for (ix, does_overlap) in self
            .range_map
            .iter()
            .map(|osrange| osrange.is_none())
            .enumerate()
        {
            if does_overlap {
                res.push(ix);
            }
        }
        res
    }

    pub fn get_range(&self, aabb: &Aabb) -> Option<SlabRange> {
        if let Some((left, right)) = self.aabb.get_crossover(aabb) {
            let start = self.get_index(&left.get_direction());
            let end = self.get_index(&right.get_direction());
            let num_slabs = self.slabs.len();
            if end < start {
                if start - end < (num_slabs / 2) {
                    Some(SlabRange {
                        start: end,
                        end: start,
                        num_slabs,
                    })
                } else {
                    Some(SlabRange {
                        start,
                        end,
                        num_slabs,
                    })
                }
            } else {
                if end - start < (num_slabs / 2) {
                    Some(SlabRange {
                        start,
                        end,
                        num_slabs,
                    })
                } else {
                    Some(SlabRange {
                        start: end,
                        end: start,
                        num_slabs,
                    })
                }
            }
        } else {
            None
        }
    }
}

impl Display for Tile {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "Tile: overlaps: ")?;
        for osrange in &self.range_map {
            if osrange.is_some() {
                write!(f, "0")?;
            } else {
                write!(f, "1")?;
            }
        }
        writeln!(f, "")?;
        writeln!(f, "Slabs:")?;
        for slab in &self.slabs {
            writeln!(f, "{}", slab)?;
        }
        write!(f, "")
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Slab {
    pub rleft: Ray,
    pub rright: Ray,
    ls: LineSegment,
    obj_indices: Vec<usize>,
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

    fn overlaps(&self, obj: &Object) -> bool {
        let geo = obj.get_geometry();
        between_rays(&obj.get_origin(), &self.rleft, &self.rright)
            || self.rleft.intersect(&geo).is_some()
            || self.rright.intersect(&geo).is_some()
            || geo.intersect(&self.ls).is_some()
    }

    pub fn object_index_iterator(&self) -> Iter<'_, usize> {
        self.obj_indices.iter()
    }

    fn insert_object(&mut self, obj_index: usize) {
        if !self.obj_indices.contains(&obj_index) {
            self.obj_indices.push(obj_index);
        }
    }

    fn remove_object(&mut self, obj_index: usize, keep_indices: bool) {
        self.obj_indices.retain(|&ix| ix != obj_index);
        if !keep_indices {
            for ix in 0..self.obj_indices.len() {
                if self.obj_indices[ix] > obj_index {
                    self.obj_indices[ix] -= 1;
                }
            }
        }
    }

    fn clear(&mut self) {
        self.obj_indices.clear();
    }
}

impl Display for Slab {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "Slab: l: ({:.2}, {:.2}), r: ({:.2}, {:.2}), indices: ",
            self.rleft.get_direction().x,
            self.rleft.get_direction().y,
            self.rright.get_direction().x,
            self.rright.get_direction().y
        )?;
        for val in self.obj_indices.iter() {
            write!(f, "{}, ", *val)?;
        }
        write!(f, "")
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
        let (t, l, b, r) = aabb.get_tlbr();
        match self {
            Quadrant::Q0 => P2::new(l, b),
            Quadrant::Q1 => P2::new(l, t),
            Quadrant::Q2 => P2::new(r, t),
            Quadrant::Q3 => P2::new(r, b),
        }
    }
}
