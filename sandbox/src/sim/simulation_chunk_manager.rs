use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    path::PathBuf,
    sync::Arc,
};

use anyhow::*;
use cgmath::{InnerSpace, Vector2};
use corrode::renderer::{create_device_image_with_usage, DeviceImageView};
use image::{ImageBuffer, Rgba};
use vulkano::{
    buffer::CpuAccessibleBuffer,
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryCommandBuffer},
    device::Queue,
    format::Format,
    image::ImageUsage,
    sync::GpuFuture,
};

use crate::{
    matter::MatterDefinitions,
    sim::{empty_u32, write_canvas_chunk_to_matter_image, write_matter_image_to_canvas_chunk},
    utils::{load_bitmap_image_from_path, BitmapImage},
    CANVAS_CHUNK_SIZE, CELL_OFFSETS_NINE, HALF_CANVAS, MAX_GPU_CHUNKS, SIM_CANVAS_SIZE,
};

pub struct WorldChunk {
    pub image: BitmapImage,
    pub gpu_chunk: Option<GpuChunk>,
}

impl WorldChunk {
    fn empty() -> WorldChunk {
        WorldChunk {
            image: BitmapImage::empty(*CANVAS_CHUNK_SIZE, *CANVAS_CHUNK_SIZE),
            gpu_chunk: None,
        }
    }

    pub fn load_from_disk(image_path: PathBuf) -> WorldChunk {
        let map_img = match load_bitmap_image_from_path(image_path) {
            std::result::Result::Ok(loaded_image) => {
                debug!("Found map image");
                loaded_image
            }
            Err(e) => {
                debug!("{}. No image. Loading empty chunk", e.to_string(),);
                BitmapImage::empty(*CANVAS_CHUNK_SIZE, *CANVAS_CHUNK_SIZE)
            }
        };
        WorldChunk {
            image: map_img,
            gpu_chunk: None,
        }
    }

    /// Adds gpu chunk to use by this world chunk and fills it with the content from Bitmap Image
    pub fn write_to_gpu(
        &mut self,
        matter_definitions: &MatterDefinitions,
        chunk: GpuChunk,
    ) -> Result<()> {
        self.gpu_chunk = Some(chunk);
        write_matter_image_to_canvas_chunk(
            &self.image,
            matter_definitions,
            self.gpu_chunk.as_ref().unwrap().get_matter_input(),
            self.gpu_chunk.as_ref().unwrap().get_matter_output(),
        )
    }

    /// Writes gpu content to Bitmap Image and returns the gpu chunk removing it from use by this world chunk
    pub fn unload_from_gpu(
        &mut self,
        matter_definitions: &MatterDefinitions,
        queue: Arc<Queue>,
    ) -> Result<GpuChunk> {
        self.image = write_canvas_chunk_to_matter_image(
            matter_definitions,
            self.gpu_chunk.as_ref().unwrap().get_matter_input(),
        )?;
        self.clear_data(queue)?;
        Ok(self.gpu_chunk.take().unwrap())
    }

    fn clear_data(&self, queue: Arc<Queue>) -> Result<()> {
        let mut builder = AutoCommandBufferBuilder::primary(
            queue.device().clone(),
            queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )?;
        let chunk = self.gpu_chunk.as_ref().unwrap();
        builder
            .clear_color_image(chunk.image.image().clone(), [0.0; 4].into())?
            .fill_buffer(chunk.objects_matter.clone(), 0)?
            .fill_buffer(chunk.objects_color.clone(), 0)?
            .fill_buffer(chunk.matter_in.clone(), 0)?
            .fill_buffer(chunk.matter_out.clone(), 0)?;
        let command_buffer = builder.build()?;
        let finished = command_buffer.execute(queue)?;
        let _fut = finished.then_signal_fence_and_flush()?;
        Ok(())
    }

    pub fn write_to_cpu(&mut self, matter_definitions: &MatterDefinitions) -> Result<()> {
        self.image = write_canvas_chunk_to_matter_image(
            matter_definitions,
            self.gpu_chunk.as_ref().unwrap().get_matter_input(),
        )?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct GpuChunk {
    pub matter_in: Arc<CpuAccessibleBuffer<[u32]>>,
    pub matter_out: Arc<CpuAccessibleBuffer<[u32]>>,
    pub objects_matter: Arc<CpuAccessibleBuffer<[u32]>>,
    pub objects_color: Arc<CpuAccessibleBuffer<[u32]>>,
    pub image: DeviceImageView,
}

impl GpuChunk {
    pub fn new(comp_queue: Arc<Queue>, format: Format) -> Result<GpuChunk> {
        let matter_in = empty_u32(
            comp_queue.device().clone(),
            (*SIM_CANVAS_SIZE * *SIM_CANVAS_SIZE) as usize,
        )?;
        let matter_out = empty_u32(
            comp_queue.device().clone(),
            (*SIM_CANVAS_SIZE * *SIM_CANVAS_SIZE) as usize,
        )?;
        let objects_matter = empty_u32(
            comp_queue.device().clone(),
            (*SIM_CANVAS_SIZE * *SIM_CANVAS_SIZE) as usize,
        )?;
        let objects_color = empty_u32(
            comp_queue.device().clone(),
            (*SIM_CANVAS_SIZE * *SIM_CANVAS_SIZE) as usize,
        )?;
        let image = create_device_image_with_usage(
            comp_queue.clone(),
            [*SIM_CANVAS_SIZE; 2],
            format,
            ImageUsage {
                sampled: true,
                storage: true,
                transfer_destination: true,
                ..ImageUsage::none()
            },
        )?;
        let mut builder = AutoCommandBufferBuilder::primary(
            comp_queue.device().clone(),
            comp_queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )?;
        builder.clear_color_image(image.image().clone(), [0.0; 4].into())?;
        let command_buffer = builder.build()?;
        let finished = command_buffer.execute(comp_queue)?;
        let _fut = finished.then_signal_fence_and_flush()?;
        Ok(GpuChunk {
            matter_in,
            matter_out,
            objects_matter,
            objects_color,
            image,
        })
    }

    pub fn get_matter_input(&self) -> Arc<CpuAccessibleBuffer<[u32]>> {
        self.matter_in.clone()
    }

    pub fn get_matter_output(&self) -> Arc<CpuAccessibleBuffer<[u32]>> {
        self.matter_out.clone()
    }
}

pub struct SimulationChunkManager {
    pub queue: Arc<Queue>,
    canvas_pos: Vector2<i32>,
    chunk_pos: Vector2<i32>,
    // An infinite amount (create as we go). They will own gpu chunks while they are in use "around player"
    world_chunks: HashMap<Vector2<i32>, WorldChunk>,
    // A finite amount (4 * 4)?
    gpu_chunk_pool: VecDeque<GpuChunk>,
    // A set of canvas coordinates currently using a gpu chunk
    pub chunks_in_use: HashSet<Vector2<i32>>,
    // Chunks that are to be written to by world interaction
    pub interaction_chunks: Vec<Vector2<i32>>,
    // Chunks that determine what we will load as player moves
    nearest_nine_chunks: HashSet<Vector2<i32>>,
    prev_nine_chunks: Option<HashSet<Vector2<i32>>>,
    // Chunks that need to be loaded
    chunks_to_load: VecDeque<Vector2<i32>>,
    chunks_to_unload: VecDeque<Vector2<i32>>,
}

impl SimulationChunkManager {
    pub fn new(comp_queue: Arc<Queue>, format: Format) -> Result<SimulationChunkManager> {
        let chunk_pos = Vector2::new(0, 0);
        let mut manager = SimulationChunkManager {
            queue: comp_queue.clone(),
            canvas_pos: Vector2::new(0, 0),
            chunk_pos,
            world_chunks: HashMap::new(),
            gpu_chunk_pool: VecDeque::new(),
            chunks_in_use: HashSet::new(),
            interaction_chunks: vec![
                Vector2::new(0, 0) + Vector2::new(0, 0),
                Vector2::new(0, 0) + Vector2::new(0, 1),
                Vector2::new(0, 0) + Vector2::new(1, 1),
                Vector2::new(0, 0) + Vector2::new(1, 0),
            ],
            nearest_nine_chunks: CELL_OFFSETS_NINE.iter().cloned().collect(),
            prev_nine_chunks: None,
            chunks_to_load: VecDeque::new(),
            chunks_to_unload: VecDeque::new(),
        };
        // Insert one world chunk
        manager.world_chunks.insert(chunk_pos, WorldChunk::empty());
        // Fill gpu chunk pool:
        for _ in 0..MAX_GPU_CHUNKS {
            manager
                .gpu_chunk_pool
                .push_back(GpuChunk::new(comp_queue.clone(), format)?);
        }

        // Take some chunks around player to use
        /*
           | x | x | x |
           | x | x | x |
           | x | x | x |
        */
        for offset in CELL_OFFSETS_NINE.iter() {
            let chunk_pos = manager.chunk_pos + offset;
            manager.chunks_to_load.push_back(chunk_pos);
        }
        Ok(manager)
    }

    /// Will panic if chunk is not in use...
    fn get_world_gpu_chunk(&self, chunk_pos: &Vector2<i32>) -> GpuChunk {
        // println!("Chunk pos {:?}", chunk_pos);
        self.world_chunks
            .get(chunk_pos)
            .unwrap()
            .gpu_chunk
            .as_ref()
            .unwrap()
            .clone()
    }

    fn get_world_gpu_chunk_mut(&mut self, chunk_pos: &Vector2<i32>) -> &mut GpuChunk {
        self.world_chunks
            .get_mut(chunk_pos)
            .unwrap()
            .gpu_chunk
            .as_mut()
            .unwrap()
    }

    pub fn get_chunks_for_compute(&self) -> (Vector2<i32>, Vec<GpuChunk>) {
        (
            self.interaction_chunks[0] * *SIM_CANVAS_SIZE as i32 - *HALF_CANVAS,
            self.interaction_chunks
                .iter()
                .map(|pos| self.get_world_gpu_chunk(pos))
                .collect(),
        )
    }

    pub fn update_compute_chunks(&mut self, chunks: Vec<GpuChunk>) {
        for (i, c) in chunks.iter().enumerate().take(4) {
            let pos = self.interaction_chunks[i];
            let gpu_chunk = self.get_world_gpu_chunk_mut(&pos);
            *gpu_chunk = c.clone();
        }
    }

    pub fn get_chunks_for_render(&self) -> Vec<(Vector2<i32>, GpuChunk)> {
        self.chunks_in_use
            .iter()
            .map(|pos| (*pos, self.get_world_gpu_chunk(pos)))
            .collect()
    }

    pub fn load_map_from_disk(
        &mut self,
        map_dir: PathBuf,
        player_pos: Vector2<i32>,
        matter_definitions: &MatterDefinitions,
    ) -> Result<()> {
        for file in fs::read_dir(&map_dir).unwrap() {
            let file = file?.file_name();
            let file_name = file.to_str().unwrap();
            let file_path = map_dir.join(file_name);
            if std::fs::metadata(&file_path).unwrap().is_file()
                && file_name.starts_with("chunk")
                && file_name.ends_with(".png")
            {
                let splits = file_name.split('.').take(1).collect::<Vec<&str>>()[0]
                    .split('_')
                    .collect::<Vec<&str>>();
                let x = splits[1].parse::<i32>().unwrap();
                let y = splits[2].parse::<i32>().unwrap();
                self.world_chunks.insert(
                    Vector2::new(x, y),
                    WorldChunk::load_from_disk(file_path.clone()),
                );
            }
        }

        // Take some chunks around player to use
        /*
           | x | x | x |
           | x | x | x |
           | x | x | x |
        */
        for offset in CELL_OFFSETS_NINE.iter() {
            let chunk_pos = self.chunk_pos + offset;
            self.chunks_to_load.push_back(chunk_pos);
        }

        self.update_chunks(player_pos, matter_definitions)?;

        Ok(())
    }

    fn load_chunks_from_queue(&mut self, matter_definitions: &MatterDefinitions) -> Result<()> {
        while !self.chunks_to_unload.is_empty() {
            let chunk_pos = self.chunks_to_unload.pop_front().unwrap();
            self.remove_gpu_chunk_from_world_use(chunk_pos, matter_definitions)?;
        }
        while !self.chunks_to_load.is_empty() {
            let chunk_pos = self.chunks_to_load.pop_front().unwrap();
            self.add_gpu_chunk_to_world_use(chunk_pos, matter_definitions)?;
        }
        Ok(())
    }

    fn remove_gpu_chunk_from_world_use(
        &mut self,
        chunk_pos: Vector2<i32>,
        matter_definitions: &MatterDefinitions,
    ) -> Result<()> {
        if let Some(world_chunk) = self.world_chunks.get_mut(&chunk_pos) {
            let gpu_chunk = world_chunk.unload_from_gpu(matter_definitions, self.queue.clone())?;
            self.chunks_in_use.remove(&chunk_pos);
            self.gpu_chunk_pool.push_back(gpu_chunk);
        } else {
            panic!(
                "World did not contain chunk at {:?} when removing gpu chunk from world use",
                chunk_pos
            );
        };
        Ok(())
    }

    fn add_gpu_chunk_to_world_use(
        &mut self,
        chunk_pos: Vector2<i32>,
        matter_definitions: &MatterDefinitions,
    ) -> Result<()> {
        if self.chunks_in_use.contains(&chunk_pos) {
            return Ok(());
        }
        let world_chunk = if let Some(world_chunk) = self.world_chunks.get_mut(&chunk_pos) {
            world_chunk
        } else {
            // If world chunk didn't exist at requested chunk pos, we just create it (empty)
            self.world_chunks.insert(chunk_pos, WorldChunk::empty());
            self.world_chunks.get_mut(&chunk_pos).unwrap()
        };
        // Write world chunk image to gpu
        let gpu_chunk = self.gpu_chunk_pool.pop_front().unwrap();
        world_chunk.write_to_gpu(matter_definitions, gpu_chunk)?;
        // Tell manager gpu chunk at index is in use
        self.chunks_in_use.insert(chunk_pos);
        Ok(())
    }

    pub fn save_one_chunk_to_disk(
        &mut self,
        map_dir: PathBuf,
        matter_definitions: &MatterDefinitions,
    ) -> Result<()> {
        let chunk_pos = Vector2::new(0, 0);
        self.world_chunks
            .get_mut(&chunk_pos)
            .unwrap()
            .write_to_cpu(matter_definitions)?;
        let chunk = self.world_chunks.get(&chunk_pos).unwrap();
        let image = ImageBuffer::<Rgba<u8>, _>::from_raw(
            *CANVAS_CHUNK_SIZE,
            *CANVAS_CHUNK_SIZE,
            &chunk.image.data[..],
        )
        .unwrap();

        let filename = format!("chunk_{}_{}.png", chunk_pos.x, chunk_pos.y);
        let image_path = map_dir.join(&filename);
        image.save(image_path).unwrap();

        Ok(())
    }

    pub fn save_chunks_to_disk(
        &mut self,
        map_dir: PathBuf,
        matter_definitions: &MatterDefinitions,
    ) -> Result<()> {
        for gpu_chunk_pos in self.chunks_in_use.iter() {
            self.world_chunks
                .get_mut(gpu_chunk_pos)
                .unwrap()
                .write_to_cpu(matter_definitions)?;
        }
        for (chunk_pos, chunk) in self.world_chunks.iter() {
            let image = ImageBuffer::<Rgba<u8>, _>::from_raw(
                *CANVAS_CHUNK_SIZE,
                *CANVAS_CHUNK_SIZE,
                &chunk.image.data[..],
            )
            .unwrap();

            let filename = format!("chunk_{}_{}.png", chunk_pos.x, chunk_pos.y);
            let image_path = map_dir.join(&filename);
            image.save(image_path).unwrap();
        }

        Ok(())
    }

    pub fn update_chunks(
        &mut self,
        player_pos: Vector2<i32>,
        matter_definitions: &MatterDefinitions,
    ) -> Result<()> {
        self.canvas_pos = player_pos;
        self.chunk_pos = Vector2::new(
            (player_pos.x as f32 / (*CANVAS_CHUNK_SIZE) as f32).round() as i32,
            (player_pos.y as f32 / (*CANVAS_CHUNK_SIZE) as f32).round() as i32,
        );
        self.interaction_chunks = self.get_nearest_four_chunks();
        self.prev_nine_chunks = Some(self.nearest_nine_chunks.clone());
        self.nearest_nine_chunks = self.get_nearest_nine_chunks();
        // if 9 chunks changed, we must load more...
        let difference: HashSet<_> = self
            .nearest_nine_chunks
            .difference(self.prev_nine_chunks.as_ref().unwrap())
            .cloned()
            .collect();
        if !difference.is_empty() {
            // If we ran out of chunks, start unloading chunks farther than one
            if difference.len() > self.gpu_chunk_pool.len() {
                self.add_farthest_chunks_for_unloading(
                    difference.len() - self.gpu_chunk_pool.len(),
                );
            }

            for chunk in difference {
                self.chunks_to_load.push_back(chunk);
            }
        }
        self.load_chunks_from_queue(matter_definitions)
    }

    fn add_farthest_chunks_for_unloading(&mut self, count: usize) {
        let mut chunks_in_use = self
            .chunks_in_use
            .clone()
            .into_iter()
            .collect::<Vec<Vector2<i32>>>();
        // Sort from farthest to closest
        chunks_in_use.sort_unstable_by(|a, b| {
            let pos_diff_a = a.cast::<f32>().unwrap() - self.chunk_pos.cast::<f32>().unwrap();
            let pos_diff_b = b.cast::<f32>().unwrap() - self.chunk_pos.cast::<f32>().unwrap();
            pos_diff_b
                .magnitude()
                .partial_cmp(&pos_diff_a.magnitude())
                .unwrap()
        });
        self.chunks_to_unload
            .extend(chunks_in_use.iter().take(count));
    }

    fn get_nearest_nine_chunks(&self) -> HashSet<Vector2<i32>> {
        CELL_OFFSETS_NINE
            .iter()
            .map(|offset| self.chunk_pos + offset)
            .collect()
    }

    ///
    /// | 2 | 3 |
    /// | 0 | 1 |
    fn get_nearest_four_chunks(&self) -> Vec<Vector2<i32>> {
        [
            vec![
                self.chunk_pos + Vector2::new(0, 0),
                self.chunk_pos + Vector2::new(1, 0),
                self.chunk_pos + Vector2::new(0, 1),
                self.chunk_pos + Vector2::new(1, 1),
            ],
            vec![
                self.chunk_pos + Vector2::new(0, -1),
                self.chunk_pos + Vector2::new(1, -1),
                self.chunk_pos + Vector2::new(0, 0),
                self.chunk_pos + Vector2::new(1, 0),
            ],
            vec![
                self.chunk_pos + Vector2::new(-1, -1),
                self.chunk_pos + Vector2::new(0, -1),
                self.chunk_pos + Vector2::new(-1, 0),
                self.chunk_pos + Vector2::new(0, 0),
            ],
            vec![
                self.chunk_pos + Vector2::new(-1, 0),
                self.chunk_pos + Vector2::new(0, 0),
                self.chunk_pos + Vector2::new(-1, 1),
                self.chunk_pos + Vector2::new(0, 1),
            ],
        ]
        .into_iter()
        .map(|option| {
            // the distance of this option from player
            let dist = option.iter().fold(0.0f32, |acc, val| {
                let chunk_pos_center = val.cast::<f32>().unwrap() * *SIM_CANVAS_SIZE as f32;
                let diff = chunk_pos_center - self.canvas_pos.cast::<f32>().unwrap();
                acc + diff.magnitude()
            }) / 4.0f32;
            (option, dist)
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap()
        .0
    }
}
