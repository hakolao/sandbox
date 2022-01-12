use std::{collections::BTreeSet, fs};

use anyhow::*;
use cgmath::Vector2;
use corrode::api::EngineApi;

use crate::{
    app::InputAction,
    map_path,
    object::{
        Angle, AngularVelocity, LinearVelocity, PixelData, PixelObjectSaveData,
        PixelObjectSaveDataArray, Position,
    },
    settings::AppSettings,
    simulation::Simulation,
    utils::get_map_directory_names,
};

pub struct EditorSaveLoader {
    pub map_name: String,
    pub map_file_names: BTreeSet<String>,
}

impl EditorSaveLoader {
    pub fn save_map(
        &mut self,
        api: &mut EngineApi<InputAction>,
        simulation: &mut Simulation,
        settings: &AppSettings,
    ) -> Result<()> {
        let EngineApi {
            ecs_world, ..
        } = api;
        let dir_path = map_path().join(&format!("{}", self.map_name));
        fs::create_dir_all(dir_path.clone()).unwrap();
        simulation.save_map_to_disk(dir_path.clone(), settings)?;

        // Save objects
        let obj_dir_path = dir_path.join("objects");
        if obj_dir_path.exists() {
            fs::remove_dir_all(obj_dir_path.clone()).unwrap();
        }
        fs::create_dir_all(obj_dir_path.clone()).unwrap();
        let mut obj_save_data = PixelObjectSaveDataArray {
            objects: vec![],
        };
        for (id, (pixel_data, pos, lin_vel, angle, ang_vel)) in &mut ecs_world.query::<(
            &PixelData,
            &Position,
            &LinearVelocity,
            &Angle,
            &AngularVelocity,
        )>() {
            let pixel_image = pixel_data.to_image();
            let obj_data = PixelObjectSaveData::from_dynamic_pixel_object(
                id,
                (pixel_data.clone(), *pos, *lin_vel, *angle, *ang_vel),
            );
            let img_path = obj_dir_path.join(&format!("{}.png", obj_data.id));
            pixel_image.save(img_path)?;
            obj_save_data.objects.push(obj_data);
        }

        let obj_data_path = obj_dir_path.join("objects.json");
        fs::write(obj_data_path, obj_save_data.serialize()).unwrap();

        self.map_file_names = get_map_directory_names()?;
        info!("Saved map {}", self.map_name);
        Ok(())
    }

    pub fn new_map(
        &mut self,
        api: &mut EngineApi<InputAction>,
        simulation: &mut Simulation,
    ) -> Result<()> {
        simulation.reset(api.renderer.image_format())?;
        api.reset_world()?;
        self.map_name = "New".to_string();
        info!("New empty map");
        Ok(())
    }

    pub fn load_map(
        &mut self,
        api: &mut EngineApi<InputAction>,
        simulation: &mut Simulation,
        map_name: &str,
    ) -> Result<()> {
        simulation.reset(api.renderer.image_format())?;
        api.reset_world()?;
        simulation.load_map_from_disk(api, map_name, Vector2::new(0, 0))?;
        self.map_name = map_name.to_string();
        info!("Loaded map {}", map_name);
        Ok(())
    }

    pub fn delete_map(&mut self, map: &str) -> Result<()> {
        let dir_path = map_path().join(&format!("{}", map));
        fs::remove_dir_all(dir_path.clone()).unwrap();
        self.map_file_names = get_map_directory_names()?;
        info!("Removed map {}", map);
        Ok(())
    }
}
