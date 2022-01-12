use std::{collections::BTreeMap, env::current_dir, fs, sync::Arc};

use anyhow::*;
use cgmath::Vector2;
use corrode::physics::PhysicsWorld;
use egui::TextureId;
use hecs::World;

use crate::{
    interact::{variated_color, CanvasDrawState},
    sim::{world_pos_inside_canvas, Simulation},
    utils::{load_image_from_file_bytes, BitmapImage},
};

pub struct EditorPlacer {
    pub object_matter: u32,
    pub place_object: String,
    pub obj_image_assets: BTreeMap<String, Arc<BitmapImage>>,
    pub object_image_texture_ids: BTreeMap<String, TextureId>,
    pub bitmap_image: Option<BitmapImage>,
}

impl EditorPlacer {
    pub fn place_object(
        &self,
        ecs_world: &mut World,
        physics_world: &mut PhysicsWorld,
        simulation: &mut Simulation,
        mouse_world_pos: Vector2<f32>,
    ) -> Result<()> {
        if world_pos_inside_canvas(mouse_world_pos, simulation.camera_pos) {
            simulation.add_dynamic_pixel_object(
                ecs_world,
                physics_world,
                self.obj_image_assets.get(&self.place_object).unwrap(),
                self.object_matter,
                Vector2::new(mouse_world_pos.x, mouse_world_pos.y),
                Vector2::new(0.0, 0.0),
                0.0,
                0.0,
            )?;
        }

        Ok(())
    }

    pub fn update_in_place_paint_object(
        &mut self,
        simulation: &mut Simulation,
        canvas_draw_state: &CanvasDrawState,
    ) {
        let max = canvas_draw_state.max.unwrap();
        let min = canvas_draw_state.min.unwrap();
        let width = max.x - min.x + 1;
        let height = max.y - min.y + 1;
        // Form bitmap image
        let mut image = BitmapImage::empty(width as u32, height as u32);
        for pixel in canvas_draw_state.pixels.iter() {
            let img_index = ((height - (pixel.y - min.y) - 1) * width + (pixel.x - min.x)) as usize;
            let matter_color = simulation.matter_definitions.definitions
                [self.object_matter as usize]
                .color
                .to_be_bytes();
            let rgba = variated_color(matter_color);
            image.data[img_index * 4] = rgba[0];
            image.data[img_index * 4 + 1] = rgba[1];
            image.data[img_index * 4 + 2] = rgba[2];
            image.data[img_index * 4 + 3] = rgba[3];
        }
        self.bitmap_image = Some(image);
    }

    pub fn place_painted_object(
        &mut self,
        ecs_world: &mut World,
        physics_world: &mut PhysicsWorld,
        simulation: &mut Simulation,
        canvas_draw_state: &CanvasDrawState,
    ) -> Result<()> {
        let image = Arc::new(self.bitmap_image.take().unwrap());
        let world_pos = canvas_draw_state.pixels_world_pos();
        let entity = simulation.add_dynamic_pixel_object(
            ecs_world,
            physics_world,
            &image,
            self.object_matter,
            world_pos,
            Vector2::new(0.0, 0.0),
            0.0,
            0.0,
        )?;
        simulation.loaded_obj_images.insert(entity.id(), image);
        Ok(())
    }
}

pub fn get_object_image_files() -> Result<BTreeMap<String, Arc<BitmapImage>>> {
    let mut object_images = BTreeMap::new();
    let dir_path = current_dir()?.join("assets/object_images");
    for file in fs::read_dir(dir_path.clone()).unwrap() {
        let file = file?.file_name();
        let file_name = file.to_str().unwrap();
        let file_path = dir_path.join(file_name);
        let contents = fs::read(file_path)?;
        let image = Arc::new(load_image_from_file_bytes(&contents));
        object_images.insert(file_name.to_string(), image);
    }
    Ok(object_images)
}
