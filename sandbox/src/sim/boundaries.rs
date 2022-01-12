use hecs::Entity;

use crate::{BITMAP_RATIO, SIM_CANVAS_SIZE};

pub struct PhysicsBoundaries {
    pub solids_changed: bool,
    pub powders_changed: bool,
    pub liquids_changed: bool,
    pub solid_bitmap: Vec<f64>,
    pub powder_bitmap: Vec<f64>,
    pub liquid_bitmap: Vec<f64>,
    pub solid_objects: Vec<Entity>,
    pub powder_objects: Vec<Entity>,
    pub liquid_objects: Vec<Entity>,
}

impl PhysicsBoundaries {
    pub fn new() -> PhysicsBoundaries {
        let bitmap_size = (*SIM_CANVAS_SIZE / *BITMAP_RATIO) as usize;
        PhysicsBoundaries {
            solids_changed: false,
            powders_changed: false,
            liquids_changed: false,
            solid_bitmap: vec![0.0; bitmap_size * bitmap_size],
            powder_bitmap: vec![0.0; bitmap_size * bitmap_size],
            liquid_bitmap: vec![0.0; bitmap_size * bitmap_size],
            solid_objects: vec![],
            powder_objects: vec![],
            liquid_objects: vec![],
        }
    }
}
