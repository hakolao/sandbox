use std::{collections::BTreeMap, env::current_dir, fs, path::PathBuf, sync::Arc};

use anyhow::*;
use cgmath::{MetricSpace, Vector2};
use corrode::{
    api::{remove_physics_entity, EngineApi},
    physics::PhysicsWorld,
    time::PerformanceTimer,
};
use hecs::{Entity, World};
use rapier2d::prelude::*;
use rayon::{
    iter::{IntoParallelIterator, ParallelIterator},
    prelude::IntoParallelRefIterator,
};
use vulkano::{device::Queue, format::Format};

use crate::{
    app::InputAction,
    map_path,
    matter::{MatterDefinition, MatterDefinitions, MatterState},
    object::{
        collider_from_convex_decomposition, dynamic_pixel_object,
        extract_connected_components_from_bitmap, form_contour_vertices,
        form_pixel_data_with_contours_from_image, invisible_sensor_object, invisible_static_object,
        update_after_physics, Angle, AngularVelocity, DeformedObjectData,
        DynamicPixelObjectCreationData, InvisibleObject, LinearVelocity, PixelData,
        PixelObjectSaveDataArray, Position, TempPixel,
    },
    settings::AppSettings,
    simulation::{
        boundaries::PhysicsBoundaries, create_boundary_object_data, get_alive_pixels,
        is_inside_sim_canvas, sim_canvas_index, sim_chunk_canvas_index, world_pos_to_canvas_pos,
        CASimulator, SimulationChunkManager,
    },
    utils::{load_image_from_file_bytes, rotate_radians, BitmapImage, CanvasMouseState},
    CELL_UNIT_SIZE, SIM_CANVAS_SIZE, WORLD_UNIT_SIZE,
};

pub struct Simulation {
    ca_simulator: CASimulator,
    pub boundaries: PhysicsBoundaries,
    pub object_pixel_query: Option<(u32, Vec<Entity>)>,

    pub camera_pos: Vector2<f32>,
    pub camera_canvas_pos: Vector2<i32>,
    pub chunk_manager: SimulationChunkManager,
    tmp_object_ids: Vec<Vec<Entity>>,
    pub loaded_obj_images: BTreeMap<u32, Arc<BitmapImage>>,

    pub matter_definitions: MatterDefinitions,

    pub obj_write_timer: PerformanceTimer,
    pub obj_read_timer: PerformanceTimer,
    pub ca_timer: PerformanceTimer,
    pub boundary_timer: PerformanceTimer,
    pub physics_timer: PerformanceTimer,
}

impl Simulation {
    pub fn new(
        comp_queue: Arc<Queue>,
        matter_definitions: MatterDefinitions,
        image_format: Format,
    ) -> Result<Simulation> {
        let mut ca_simulator = CASimulator::new(comp_queue.clone(), matter_definitions.empty)?;
        ca_simulator.update_matter_data(&matter_definitions)?;
        let tmp_object_ids: Vec<Vec<Entity>> =
            vec![vec![]; (*SIM_CANVAS_SIZE * *SIM_CANVAS_SIZE) as usize];

        Ok(Simulation {
            ca_simulator,
            boundaries: PhysicsBoundaries::new(),
            object_pixel_query: None,
            camera_pos: Vector2::new(0.0, 0.0),
            camera_canvas_pos: Vector2::new(0, 0),
            chunk_manager: SimulationChunkManager::new(comp_queue, image_format)?,
            tmp_object_ids,
            loaded_obj_images: BTreeMap::new(),
            matter_definitions,
            obj_write_timer: PerformanceTimer::new(),
            obj_read_timer: PerformanceTimer::new(),
            ca_timer: PerformanceTimer::new(),
            boundary_timer: PerformanceTimer::new(),
            physics_timer: PerformanceTimer::new(),
        })
    }

    pub fn reset(&mut self, image_format: Format) -> Result<()> {
        *self = Simulation::new(
            self.chunk_manager.queue.clone(),
            self.matter_definitions.clone(),
            image_format,
        )?;
        Ok(())
    }

    /*
    1. Write objects to CA grid
    2. Step CA (multiple steps if needed). Updates solid etc. bitmaps
    3. Form contours & physics boundaries from CA Grid
    4. Step physics simulation
    */
    pub fn step(
        &mut self,
        api: &mut EngineApi<InputAction>,
        settings: AppSettings,
        canvas_mouse_state: &CanvasMouseState,
    ) -> Result<()> {
        // If we intend to move in the world via chunked simulation
        if settings.chunked_simulation {
            self.camera_pos = api.main_camera.pos();
        }
        self.camera_canvas_pos = {
            let canvas_pos_f32 = world_pos_to_canvas_pos(self.camera_pos);
            Vector2::new(canvas_pos_f32.x as i32, canvas_pos_f32.y as i32)
        };

        self.chunk_manager
            .update_chunks(self.camera_canvas_pos, &self.matter_definitions)?;

        self.obj_write_timer.start();
        self.write_pixel_objects_to_grid(api)?;
        self.obj_write_timer.time_it();

        self.ca_timer.start();
        self.ca_simulator
            .step(settings, self.camera_canvas_pos, &mut self.chunk_manager)?;
        self.ca_timer.time_it();

        self.object_pixel_query = self.query_object(canvas_mouse_state.mouse_on_canvas)?;

        self.obj_read_timer.start();
        self.update_objects_from_grid(api)?;
        self.obj_read_timer.time_it();

        self.boundary_timer.start();
        self.update_physics_boundaries(api)?;
        self.boundary_timer.time_it();

        self.physics_timer.start();
        api.physics_world
            .step(&api.thread_pool, |_intersect_event| {}, |_contact_event| {});
        self.update_dynamic_physics_objects(api)?;
        self.physics_timer.time_it();

        Ok(())
    }

    fn update_dynamic_physics_objects(&mut self, api: &mut EngineApi<InputAction>) -> Result<()> {
        let EngineApi {
            ecs_world,
            physics_world,
            ..
        } = api;
        let mut remove = vec![];
        for (id, (rb, pos, lin_vel, angle, ang_vel)) in ecs_world.query_mut::<(
            &RigidBodyHandle,
            &mut Position,
            &mut LinearVelocity,
            &mut Angle,
            &mut AngularVelocity,
        )>() {
            let rigid_body: &mut RigidBody = &mut physics_world.physics.bodies[*rb];
            update_after_physics(
                rigid_body,
                &mut pos.0,
                &mut lin_vel.0,
                &mut angle.0,
                &mut ang_vel.0,
            );
            if pos.0.y < -10.0 * WORLD_UNIT_SIZE {
                remove.push(id)
            }
        }
        // ToDo: Delete dropped objects
        for e in remove {
            remove_physics_entity(ecs_world, physics_world, e);
            info!("Removed physics entity {} as it dropped too far", e.id());
        }
        Ok(())
    }

    pub fn save_matter_definitions(&self) {
        let matter_definitions_path = current_dir()
            .unwrap()
            .join(&format!("assets/matter_definitions.json"));
        fs::write(matter_definitions_path, self.matter_definitions.serialize()).unwrap();
        info!("Saved matter definitions to assets/matter_definitions.json");
    }

    pub fn remove_matter_definition(&mut self, id: u32) -> Result<()> {
        assert_ne!(self.matter_definitions.empty, id);
        let definition = &self.matter_definitions.definitions[id as usize];
        info!(
            "Remove matter {}: name: {}, state: {}",
            id, definition.name, definition.state
        );
        self.matter_definitions.definitions.remove(id as usize);
        // Update ids...
        for (i, def) in self.matter_definitions.definitions.iter_mut().enumerate() {
            def.id = i as u32;
        }
        self.ca_simulator
            .update_matter_data(&self.matter_definitions)?;
        Ok(())
    }

    pub fn add_matter_to_definitions(&mut self, matter_definition: MatterDefinition) -> Result<()> {
        let id = matter_definition.id;
        if id == self.matter_definitions.definitions.len() as u32 {
            info!(
                "Add matter {}: name: {}, state: {}",
                id, matter_definition.name, matter_definition.state
            );
            self.matter_definitions.definitions.push(matter_definition);
            self.ca_simulator
                .update_matter_data(&self.matter_definitions)?;
        } else {
            info!(
                "Update matter {}: name: {}, state: {}",
                id, matter_definition.name, matter_definition.state
            );
            self.matter_definitions.definitions[id as usize] = matter_definition;
            self.ca_simulator
                .update_matter_data(&self.matter_definitions)?;
        }
        Ok(())
    }

    pub fn load_map_from_disk(
        &mut self,
        api: &mut EngineApi<InputAction>,
        map_name: &str,
        player_pos: Vector2<i32>,
    ) -> Result<()> {
        let map_path = map_path().join(&format!("{}", map_name));
        self.chunk_manager.load_map_from_disk(
            map_path.clone(),
            player_pos,
            &self.matter_definitions,
        )?;

        // Load objects
        self.loaded_obj_images.clear();
        let obj_dir_path = map_path.join("objects");
        let obj_save_data_path = obj_dir_path.join("objects.json");
        let object_save_data_str = fs::read_to_string(obj_save_data_path).unwrap();
        let object_save_data = PixelObjectSaveDataArray::deserialize(&object_save_data_str);
        for object_data in object_save_data.objects.iter() {
            let img_path = obj_dir_path.join(&format!("{}.png", object_data.id));
            let contents = fs::read(img_path.clone()).unwrap();
            let obj_img = Arc::new(load_image_from_file_bytes(&contents));
            let entity = object_data.add_dynamic_pixel_object(
                &mut api.ecs_world,
                &mut api.physics_world,
                self,
                &obj_img,
            )?;
            self.loaded_obj_images.insert(entity.id(), obj_img);
        }
        Ok(())
    }

    pub fn save_map_to_disk(&mut self, map_path: PathBuf, settings: &AppSettings) -> Result<()> {
        if settings.chunked_simulation {
            self.chunk_manager
                .save_chunks_to_disk(map_path, &self.matter_definitions)
        } else {
            self.chunk_manager
                .save_one_chunk_to_disk(map_path, &self.matter_definitions)
        }
    }

    pub fn paint_round(
        &mut self,
        line: &Vec<Vector2<i32>>,
        matter: u32,
        radius: f32,
    ) -> Result<()> {
        for &pos in line.iter() {
            if !is_inside_sim_canvas(pos, self.camera_canvas_pos) {
                continue;
            }
            let (chunk_start, grids) = self.chunk_manager.get_chunks_for_compute();
            let mut grids = [
                grids[0].matter_in.write()?,
                grids[1].matter_in.write()?,
                grids[2].matter_in.write()?,
                grids[3].matter_in.write()?,
            ];
            let y_start = pos.y - radius as i32;
            let y_end = pos.y + radius as i32;
            let x_start = pos.x - radius as i32;
            let x_end = pos.x + radius as i32;
            for y in y_start..=y_end {
                for x in x_start..=x_end {
                    if Vector2::new(x as f32, y as f32)
                        .distance(Vector2::new(pos.x as f32, pos.y as f32))
                        .round()
                        <= radius
                    {
                        let canvas_pos = Vector2::new(x, y);
                        if is_inside_sim_canvas(canvas_pos, self.camera_canvas_pos) {
                            let (chunk_index, grid_index) =
                                sim_chunk_canvas_index(canvas_pos, chunk_start);
                            if grids[chunk_index][grid_index] == self.matter_definitions.empty
                                || matter == self.matter_definitions.empty
                            {
                                grids[chunk_index][grid_index] = matter;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn paint_square(&mut self, line: &Vec<Vector2<i32>>, matter: u32, size: i32) -> Result<()> {
        for &pos in line.iter() {
            if !is_inside_sim_canvas(pos, self.camera_canvas_pos) {
                continue;
            }
            let (chunk_start, grids) = self.chunk_manager.get_chunks_for_compute();
            let mut grids = [
                grids[0].matter_in.write()?,
                grids[1].matter_in.write()?,
                grids[2].matter_in.write()?,
                grids[3].matter_in.write()?,
            ];
            let y_start = pos.y - size / 2;
            let y_end = pos.y + size / 2;
            let x_start = pos.x - size / 2;
            let x_end = pos.x + size / 2;
            for y in y_start..y_end {
                for x in x_start..x_end {
                    let canvas_pos = Vector2::new(x, y);
                    if is_inside_sim_canvas(canvas_pos, self.camera_canvas_pos) {
                        let (chunk_index, grid_index) =
                            sim_chunk_canvas_index(canvas_pos, chunk_start);
                        if grids[chunk_index][grid_index] == self.matter_definitions.empty
                            || matter == self.matter_definitions.empty
                        {
                            grids[chunk_index][grid_index] = matter;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Query cell via GUI, this should be performed on grid_next
    pub fn query_matter(&self, mouse_pos: Vector2<i32>) -> Result<Option<u32>> {
        if !is_inside_sim_canvas(mouse_pos, self.camera_canvas_pos) {
            return Ok(None);
        }
        let (chunk_start, chunks) = self.chunk_manager.get_chunks_for_compute();
        let matters = [
            chunks[0].matter_in.read()?,
            chunks[1].matter_in.read()?,
            chunks[2].matter_in.read()?,
            chunks[3].matter_in.read()?,
        ];
        let (chunk_index, grid_index) = sim_chunk_canvas_index(mouse_pos, chunk_start);
        Ok(Some(matters[chunk_index][grid_index]))
    }

    fn query_object(&self, mouse_pos: Vector2<i32>) -> Result<Option<(u32, Vec<Entity>)>> {
        if !is_inside_sim_canvas(mouse_pos, self.camera_canvas_pos) {
            return Ok(None);
        }
        let (chunk_start, chunks) = self.chunk_manager.get_chunks_for_compute();
        let obj_matters = [
            chunks[0].objects_matter.read()?,
            chunks[1].objects_matter.read()?,
            chunks[2].objects_matter.read()?,
            chunks[3].objects_matter.read()?,
        ];
        let (chunk_index, grid_index) = sim_chunk_canvas_index(mouse_pos, chunk_start);
        if obj_matters[chunk_index][grid_index] == self.matter_definitions.empty {
            Ok(None)
        } else {
            let object_ids =
                self.tmp_object_ids[sim_canvas_index(mouse_pos, self.camera_canvas_pos)].clone();
            Ok(Some((obj_matters[chunk_index][grid_index], object_ids)))
        }
    }

    pub fn write_pixel_objects_to_grid(&mut self, api: &mut EngineApi<InputAction>) -> Result<()> {
        let EngineApi {
            ecs_world, ..
        } = api;
        let (chunk_start, chunks) = self.chunk_manager.get_chunks_for_compute();
        let mut obj_matters = [
            chunks[0].objects_matter.write()?,
            chunks[1].objects_matter.write()?,
            chunks[2].objects_matter.write()?,
            chunks[3].objects_matter.write()?,
        ];
        let mut obj_colors = [
            chunks[0].objects_color.write()?,
            chunks[1].objects_color.write()?,
            chunks[2].objects_color.write()?,
            chunks[3].objects_color.write()?,
        ];
        for (id, (pixel_data, temp_canvas_pixels, pos, angle)) in
            ecs_world.query_mut::<(&PixelData, &mut Vec<TempPixel>, &mut Position, &mut Angle)>()
        {
            *temp_canvas_pixels = get_alive_pixels(pixel_data, pos.0, angle.0, id);
            for &tmp_pixel in temp_canvas_pixels.iter() {
                if is_inside_sim_canvas(tmp_pixel.canvas_pos, self.camera_canvas_pos) {
                    let (chunk_index, grid_index) =
                        sim_chunk_canvas_index(tmp_pixel.canvas_pos, chunk_start);
                    obj_matters[chunk_index][grid_index] = tmp_pixel.matter;
                    obj_colors[chunk_index][grid_index] = tmp_pixel.color;
                    self.tmp_object_ids
                        [sim_canvas_index(tmp_pixel.canvas_pos, self.camera_canvas_pos)]
                    .push(tmp_pixel.entity);
                }
            }
        }
        Ok(())
    }

    /// 1. Compare temp pixels that were written to canvas before ca simulation now after simulation
    /// 2. If they changed, object is determined to be deformed
    /// 3. Update object...
    pub fn update_objects_from_grid(&mut self, api: &mut EngineApi<InputAction>) -> Result<()> {
        let deformed_objects = self.get_deformed_object_bitmaps(api)?;
        self.clear_object_pixels_from_grid(api)?;
        self.add_deformed_objects_to_world(api, deformed_objects)?;
        Ok(())
    }

    // For each object that was deemed deformed (or to remove), create new objects
    // based on their bitmaps
    fn add_deformed_objects_to_world(
        &mut self,
        api: &mut EngineApi<InputAction>,
        deformed_objects: Vec<DeformedObjectData>,
    ) -> Result<()> {
        let EngineApi {
            ecs_world,
            physics_world,
            ..
        } = api;
        // Calculate objects
        let new_objects_data: Vec<(Entity, RigidBodyHandle, Vec<DynamicPixelObjectCreationData>)> =
            deformed_objects
                .into_par_iter()
                .map(
                    |(obj_id, rb, pixel_data, pos, lin_vel, angle, ang_vel, bitmap)| {
                        if bitmap.is_empty() {
                            (obj_id, rb, vec![])
                        } else {
                            let old_local_center = Vector2::new(
                                pixel_data.width as f32 * 0.5,
                                pixel_data.height as f32 * 0.5,
                            );
                            let new_bitmaps = extract_connected_components_from_bitmap(
                                &bitmap,
                                pixel_data.width,
                                pixel_data.height,
                            );
                            // New deformed object contours and colliders
                            let add_objects_data = new_bitmaps
                                .into_iter()
                                .map(|(bitmap, width, height, mins)| {
                                    let new_center_inside_old = Vector2::new(
                                        mins.x as f32 + width as f32 * 0.5,
                                        mins.y as f32 + height as f32 * 0.5,
                                    );
                                    let pixel_diff = new_center_inside_old - old_local_center;
                                    // Pos offset (in world units) is the difference between new shape and center of old shape
                                    let pos_offset =
                                        rotate_radians(pixel_diff * *CELL_UNIT_SIZE, angle.0);

                                    let pixel_data = PixelData::split_by_bitmap(
                                        self.matter_definitions.empty,
                                        &pixel_data,
                                        &bitmap,
                                        width,
                                        height,
                                        mins,
                                    );
                                    let contours = form_contour_vertices(
                                        &bitmap,
                                        width,
                                        height,
                                        *CELL_UNIT_SIZE as f64,
                                    );
                                    let pos = pos.0 + pos_offset;
                                    let colliders = contours
                                        .iter()
                                        .filter_map(|ring| {
                                            if ring.len() > 2 {
                                                Some(collider_from_convex_decomposition(ring))
                                            } else {
                                                None
                                            }
                                        })
                                        .collect::<Vec<Collider>>();

                                    (pixel_data, pos, lin_vel.0, angle.0, ang_vel.0, colliders)
                                })
                                .filter(|(_, _, _, _, _, colliders)| colliders.len() > 0)
                                .collect::<Vec<DynamicPixelObjectCreationData>>();
                            (obj_id, rb, add_objects_data)
                        }
                    },
                )
                .collect();
        // Add to world & physics
        for (prev_obj, rb, add_objects) in new_objects_data {
            if add_objects.is_empty() {
                physics_world.remove_physics(rb);
                ecs_world.despawn(prev_obj)?;
            } else {
                physics_world.remove_physics(rb);
                // Create new (first should retain the id)
                let mut count = 0;
                for (pixel_data, pos, lin_vel, angle, ang_vel, colliders) in add_objects {
                    let id = if count == 0 {
                        prev_obj
                    } else {
                        ecs_world.reserve_entity()
                    };
                    ecs_world.insert(
                        id,
                        dynamic_pixel_object(
                            id,
                            &mut physics_world.physics,
                            pixel_data,
                            pos,
                            lin_vel,
                            angle,
                            ang_vel,
                            colliders,
                        ),
                    )?;
                    count += 1;
                }
            }
        }
        Ok(())
    }

    /// Return calculated bitmaps (and object data) determined by how the object was deformed over ca simulation
    fn get_deformed_object_bitmaps(
        &self,
        api: &mut EngineApi<InputAction>,
    ) -> Result<Vec<DeformedObjectData>> {
        let EngineApi {
            ecs_world, ..
        } = api;
        let (chunk_start, chunks) = self.chunk_manager.get_chunks_for_compute();
        let obj_matters = [
            chunks[0].objects_matter.read()?,
            chunks[1].objects_matter.read()?,
            chunks[2].objects_matter.read()?,
            chunks[3].objects_matter.read()?,
        ];
        let obj_ids = &self.tmp_object_ids;
        let mut objects_to_check = vec![];
        for (id, (rb, pixel_data, temp_canvas_pixels, pos, lin_vel, angle, ang_vel)) in
            &mut ecs_world.query::<(
                &RigidBodyHandle,
                &PixelData,
                &Vec<TempPixel>,
                &Position,
                &LinearVelocity,
                &Angle,
                &AngularVelocity,
            )>()
        {
            objects_to_check.push((
                id,
                *rb,
                pixel_data.clone(),
                temp_canvas_pixels.clone(),
                *pos,
                *lin_vel,
                *angle,
                *ang_vel,
            ));
        }
        let deformed_objects = objects_to_check
            .into_par_iter()
            .filter_map(
                |(id, rb, pixel_data, temp_canvas_pixels, pos, lin_vel, angle, ang_vel)| {
                    let mut bitmap = vec![0.0; (pixel_data.width * pixel_data.height) as usize];
                    let mut should_update_object = false;
                    let mut pixel_count = temp_canvas_pixels.len();
                    for &tmp_pixel in temp_canvas_pixels.iter() {
                        // Only look inside canvas, deformation can only take place inside it
                        if is_inside_sim_canvas(tmp_pixel.canvas_pos, self.camera_canvas_pos) {
                            let canvas_index =
                                sim_canvas_index(tmp_pixel.canvas_pos, self.camera_canvas_pos);
                            let obj_id_in_grid =
                                obj_ids[canvas_index].iter().position(|&id| id == id);
                            // If object exists in visible canvas grid, mark bitmap 1.0. Else objet should be updated (deformed)
                            let (chunk_index, grid_index) =
                                sim_chunk_canvas_index(tmp_pixel.canvas_pos, chunk_start);
                            if obj_id_in_grid.is_some()
                                && obj_matters[chunk_index][grid_index]
                                    != self.matter_definitions.empty
                            {
                                bitmap[tmp_pixel.pixel_index] = 1.0;
                            } else {
                                pixel_count -= 1;
                                should_update_object = true;
                            }
                        }
                    }
                    if pixel_count <= 4 {
                        Some((
                            id,
                            rb,
                            pixel_data.clone(),
                            pos,
                            lin_vel,
                            angle,
                            ang_vel,
                            vec![],
                        ))
                    } else if should_update_object {
                        Some((
                            id,
                            rb,
                            pixel_data.clone(),
                            pos,
                            lin_vel,
                            angle,
                            ang_vel,
                            bitmap,
                        ))
                    } else {
                        None
                    }
                },
            )
            .collect();
        Ok(deformed_objects)
    }

    /// Clear temp pixels from objects (which are rewritten next frame).
    /// You could clear the buffers all at once, but it is faster this way... (because objects never cover the whole grid).
    /// Tried memsetting whole buffer or making a specific clear kernel...
    fn clear_object_pixels_from_grid(&mut self, api: &mut EngineApi<InputAction>) -> Result<()> {
        let EngineApi {
            ecs_world, ..
        } = api;
        let (chunk_start, chunks) = self.chunk_manager.get_chunks_for_compute();
        let mut obj_matters = [
            chunks[0].objects_matter.write()?,
            chunks[1].objects_matter.write()?,
            chunks[2].objects_matter.write()?,
            chunks[3].objects_matter.write()?,
        ];
        let mut obj_colors = [
            chunks[0].objects_color.write()?,
            chunks[1].objects_color.write()?,
            chunks[2].objects_color.write()?,
            chunks[3].objects_color.write()?,
        ];
        for (_id, temp_canvas_pixels) in &mut ecs_world.query::<&mut Vec<TempPixel>>() {
            for &tmp_pixel in temp_canvas_pixels.iter() {
                if is_inside_sim_canvas(tmp_pixel.canvas_pos, self.camera_canvas_pos) {
                    let (chunk_index, grid_index) =
                        sim_chunk_canvas_index(tmp_pixel.canvas_pos, chunk_start);
                    let canvas_index =
                        sim_canvas_index(tmp_pixel.canvas_pos, self.camera_canvas_pos);
                    obj_matters[chunk_index][grid_index] = 0x0;
                    obj_colors[chunk_index][grid_index] = 0x0;
                    if let Some(pos) = self.tmp_object_ids[canvas_index]
                        .iter()
                        .position(|x| *x == tmp_pixel.entity)
                    {
                        self.tmp_object_ids[canvas_index].swap_remove(pos);
                    }
                }
            }
            temp_canvas_pixels.clear();
        }
        Ok(())
    }

    pub fn update_physics_boundaries(&mut self, api: &mut EngineApi<InputAction>) -> Result<()> {
        let EngineApi {
            ecs_world,
            physics_world,
            ..
        } = api;
        self.ca_simulator.update_bitmaps(
            &mut self.boundaries.solid_bitmap,
            &mut self.boundaries.powder_bitmap,
            &mut self.boundaries.liquid_bitmap,
            &mut self.boundaries.solids_changed,
            &mut self.boundaries.powders_changed,
            &mut self.boundaries.liquids_changed,
        )?;

        let mut changed_bitmaps = vec![];
        let mut remove_objects = vec![];
        if self.boundaries.solids_changed {
            // Remove old objects
            remove_objects.extend(&self.boundaries.solid_objects);
            self.boundaries.solid_objects.clear();
            // Set creation to occur
            changed_bitmaps.push((&self.boundaries.solid_bitmap, MatterState::Solid));
            self.boundaries.solids_changed = false;
        }
        if self.boundaries.powders_changed {
            remove_objects.extend(&self.boundaries.powder_objects);
            self.boundaries.powder_objects.clear();
            changed_bitmaps.push((&self.boundaries.powder_bitmap, MatterState::Powder));
            self.boundaries.powders_changed = false;
        }
        if self.boundaries.liquids_changed {
            remove_objects.extend(&self.boundaries.liquid_objects);
            self.boundaries.liquid_objects.clear();
            changed_bitmaps.push((&self.boundaries.liquid_bitmap, MatterState::Liquid));
            self.boundaries.liquids_changed = false;
        }

        // Create boundary object data (with par iters) (creates colliders etc...)
        let add_objects_data = changed_bitmaps
            .par_iter()
            .map(|(bitmap, state)| {
                (
                    create_boundary_object_data(
                        self.camera_pos,
                        *bitmap,
                        *state == MatterState::Liquid,
                    ),
                    *state,
                )
            })
            .collect::<Vec<(Vec<(Vector2<f32>, f32, Collider)>, MatterState)>>();

        // remove previous boundary objects
        for e in remove_objects {
            let rb = *ecs_world.get::<RigidBodyHandle>(e).unwrap();
            physics_world.remove_physics(rb);
            ecs_world.despawn(e)?;
        }

        // Create new objects & update boundary data
        let add_objects = add_objects_data
            .into_iter()
            .map(|(obj_data, state)| {
                obj_data
                    .into_iter()
                    .map(|(pos, angle, collider)| {
                        let id = ecs_world.reserve_entity();
                        if state == MatterState::Liquid {
                            (
                                id,
                                invisible_sensor_object(
                                    id,
                                    &mut physics_world.physics,
                                    pos,
                                    angle,
                                    vec![collider],
                                ),
                                state,
                            )
                        } else {
                            (
                                id,
                                invisible_static_object(
                                    id,
                                    &mut physics_world.physics,
                                    pos,
                                    angle,
                                    vec![collider],
                                ),
                                state,
                            )
                        }
                    })
                    .collect::<Vec<(Entity, InvisibleObject, MatterState)>>()
            })
            .flatten()
            .collect::<Vec<(Entity, InvisibleObject, MatterState)>>();
        for (entity, o_components, state) in add_objects {
            match state {
                MatterState::Liquid => self.boundaries.liquid_objects.push(entity),
                MatterState::Solid => self.boundaries.solid_objects.push(entity),
                MatterState::Powder => self.boundaries.powder_objects.push(entity),
                _ => (),
            }
            api.ecs_world.insert(entity, o_components)?;
        }
        Ok(())
    }

    pub fn add_dynamic_pixel_object(
        &mut self,
        ecs_world: &mut World,
        physics_world: &mut PhysicsWorld,
        image: &Arc<BitmapImage>,
        matter: u32,
        pos: Vector2<f32>,
        lin_vel: Vector2<f32>,
        angle: f32,
        ang_vel: f32,
    ) -> Result<Entity> {
        let (pixel_data, contours) =
            form_pixel_data_with_contours_from_image(image, matter, self.matter_definitions.empty);
        let colliders = contours
            .iter()
            .map(|ring| collider_from_convex_decomposition(ring))
            .collect::<Vec<Collider>>();
        let entity = ecs_world.reserve_entity();
        ecs_world.insert(
            entity,
            dynamic_pixel_object(
                entity,
                &mut physics_world.physics,
                pixel_data,
                pos,
                lin_vel,
                angle,
                ang_vel,
                colliders,
            ),
        )?;
        Ok(entity)
    }
}
