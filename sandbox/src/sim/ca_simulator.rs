use std::{sync::Arc, time::Instant};

use anyhow::*;
use cgmath::Vector2;
use vulkano::{
    buffer::CpuAccessibleBuffer,
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer,
        PrimaryCommandBuffer,
    },
    descriptor_set::{
        layout::{DescriptorDesc, DescriptorSetLayout, DescriptorType},
        PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::Queue,
    pipeline::{ComputePipeline, Pipeline, PipelineBindPoint, PipelineLayout},
    shader::ShaderStages,
    sync::GpuFuture,
};

use crate::{
    matter::{MatterDefinition, MatterDefinitions, MatterState, MAX_TRANSITIONS},
    settings::AppSettings,
    sim::{empty_f32, empty_u32, GpuChunk, SimulationChunkManager},
    utils::u32_rgba_to_u32_abgr,
    BITMAP_RATIO, KERNEL_SIZE, MAX_NUM_MATTERS, SIM_CANVAS_SIZE,
};

pub struct CASimulator {
    pub comp_queue: Arc<Queue>,
    // Simulation pipelines (Could also be one pipeline with multiple entry points... :D)
    fall_empty_pipeline: Arc<ComputePipeline>,
    fall_swap_pipeline: Arc<ComputePipeline>,
    rise_empty_pipeline: Arc<ComputePipeline>,
    rise_swap_pipeline: Arc<ComputePipeline>,
    slide_down_empty_pipeline: Arc<ComputePipeline>,
    slide_down_swap_pipeline: Arc<ComputePipeline>,
    horizontal_empty_pipeline: Arc<ComputePipeline>,
    horizontal_swap_pipeline: Arc<ComputePipeline>,
    react_pipeline: Arc<ComputePipeline>,
    color_pipeline: Arc<ComputePipeline>,
    // Utility pipelines
    init_pipeline: Arc<ComputePipeline>,
    update_bitmap_pipeline: Arc<ComputePipeline>,
    finish_pipeline: Arc<ComputePipeline>,
    // Shader matter inputs
    matter_color_input: Arc<CpuAccessibleBuffer<[u32]>>,
    matter_state_input: Arc<CpuAccessibleBuffer<[u32]>>,
    matter_weight_input: Arc<CpuAccessibleBuffer<[f32]>>,
    matter_dispersion_input: Arc<CpuAccessibleBuffer<[u32]>>,
    matter_characteristics_input: Arc<CpuAccessibleBuffer<[u32]>>,
    matter_reaction_with_input: Arc<CpuAccessibleBuffer<[u32]>>,
    matter_reaction_direction_input: Arc<CpuAccessibleBuffer<[u32]>>,
    matter_reaction_probability_input: Arc<CpuAccessibleBuffer<[f32]>>,
    matter_reaction_transition_input: Arc<CpuAccessibleBuffer<[u32]>>,
    bitmap: Arc<CpuAccessibleBuffer<[u32]>>,
    tmp_matter: Arc<CpuAccessibleBuffer<[u32]>>,
    //... push constants
    pub sim_steps: usize,
    dispersion_step: u32,
    dispersion_dir: u32,
    move_step: u32,
    sim_pos_offset: Vector2<i32>,
    seed: f32,
    start: Instant,
}

impl CASimulator {
    pub fn new(comp_queue: Arc<Queue>, empty: u32) -> Result<CASimulator> {
        assert_eq!(*SIM_CANVAS_SIZE % KERNEL_SIZE, 0);

        let matter_color_input = empty_u32(comp_queue.device().clone(), MAX_NUM_MATTERS as usize)?;
        let matter_state_input = empty_u32(comp_queue.device().clone(), MAX_NUM_MATTERS as usize)?;
        let matter_weight_input = empty_f32(comp_queue.device().clone(), MAX_NUM_MATTERS as usize)?;
        let matter_dispersion_input =
            empty_u32(comp_queue.device().clone(), MAX_NUM_MATTERS as usize)?;
        let matter_characteristics_input =
            empty_u32(comp_queue.device().clone(), MAX_NUM_MATTERS as usize)?;
        let matter_reaction_with_input = empty_u32(
            comp_queue.device().clone(),
            MAX_NUM_MATTERS as usize * MAX_TRANSITIONS as usize,
        )?;
        let matter_reaction_direction_input = empty_u32(
            comp_queue.device().clone(),
            MAX_NUM_MATTERS as usize * MAX_TRANSITIONS as usize,
        )?;
        let matter_reaction_probability_input = empty_f32(
            comp_queue.device().clone(),
            MAX_NUM_MATTERS as usize * MAX_TRANSITIONS as usize,
        )?;
        let matter_reaction_transition_input = empty_u32(
            comp_queue.device().clone(),
            MAX_NUM_MATTERS as usize * MAX_TRANSITIONS as usize,
        )?;

        let bitmap = empty_u32(
            comp_queue.device().clone(),
            ((*SIM_CANVAS_SIZE / *BITMAP_RATIO) * (*SIM_CANVAS_SIZE / *BITMAP_RATIO)) as usize,
        )?;
        let tmp_matter = empty_u32(
            comp_queue.device().clone(),
            (*SIM_CANVAS_SIZE * *SIM_CANVAS_SIZE) as usize,
        )?;
        let spec_const = init_cs::SpecializationConstants {
            empty,
            sim_canvas_size: *SIM_CANVAS_SIZE as i32,
            bitmap_ratio: *BITMAP_RATIO as i32,
            state_empty: MatterState::Empty as u32,
            state_powder: MatterState::Powder as u32,
            state_liquid: MatterState::Liquid as u32,
            state_solid: MatterState::Solid as u32,
            state_solid_gravity: MatterState::SolidGravity as u32,
            state_gas: MatterState::Gas as u32,
            state_energy: MatterState::Energy as u32,
            state_object: MatterState::Object as u32,
            constant_11: KERNEL_SIZE,
            constant_12: KERNEL_SIZE,
        };

        fn storage_buffer_desc() -> DescriptorDesc {
            DescriptorDesc {
                ty: DescriptorType::StorageBuffer,
                descriptor_count: 1,
                variable_count: false,
                stages: ShaderStages::all(),
                immutable_samplers: Vec::new(),
            }
        }

        fn image_desc_set() -> DescriptorDesc {
            DescriptorDesc {
                ty: DescriptorType::StorageImage,
                descriptor_count: 1,
                variable_count: false,
                stages: ShaderStages::all(),
                immutable_samplers: Vec::new(),
            }
        }

        let sim_pc_requirements = {
            let shader = fall_empty_cs::load(comp_queue.device().clone())?;
            shader
                .entry_point("main")
                .unwrap()
                .push_constant_requirements()
                .cloned()
        };
        // See compute_shaders/simulation/includes.glsl for layout
        let sim_set_layout = DescriptorSetLayout::new(comp_queue.device().clone(), [
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(image_desc_set()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(image_desc_set()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(image_desc_set()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(image_desc_set()),
        ])?;
        let sim_pipeline_layout = PipelineLayout::new(
            comp_queue.device().clone(),
            [sim_set_layout],
            sim_pc_requirements,
        )?;

        let utils_pc_requirements = {
            let shader = init_cs::load(comp_queue.device().clone())?;
            shader
                .entry_point("main")
                .unwrap()
                .push_constant_requirements()
                .cloned()
        };

        // See compute_shaders/utils/includes.glsl for layout
        let utils_set_layout = DescriptorSetLayout::new(comp_queue.device().clone(), [
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
            Some(storage_buffer_desc()),
        ])?;

        let utils_pipeline_layout = PipelineLayout::new(
            comp_queue.device().clone(),
            [utils_set_layout],
            utils_pc_requirements,
        )?;

        let fall_empty_pipeline = {
            let shader = fall_empty_cs::load(comp_queue.device().clone())?;
            ComputePipeline::with_pipeline_layout(
                comp_queue.device().clone(),
                shader.entry_point("main").unwrap(),
                &spec_const,
                sim_pipeline_layout.clone(),
                None,
            )?
        };
        let fall_swap_pipeline = {
            let shader = fall_swap_cs::load(comp_queue.device().clone())?;
            ComputePipeline::with_pipeline_layout(
                comp_queue.device().clone(),
                shader.entry_point("main").unwrap(),
                &spec_const,
                sim_pipeline_layout.clone(),
                None,
            )?
        };
        let rise_empty_pipeline = {
            let shader = rise_empty_cs::load(comp_queue.device().clone())?;
            ComputePipeline::with_pipeline_layout(
                comp_queue.device().clone(),
                shader.entry_point("main").unwrap(),
                &spec_const,
                sim_pipeline_layout.clone(),
                None,
            )?
        };
        let rise_swap_pipeline = {
            let shader = rise_swap_cs::load(comp_queue.device().clone())?;
            ComputePipeline::with_pipeline_layout(
                comp_queue.device().clone(),
                shader.entry_point("main").unwrap(),
                &spec_const,
                sim_pipeline_layout.clone(),
                None,
            )?
        };
        let slide_down_empty_pipeline = {
            let shader = slide_down_empty_cs::load(comp_queue.device().clone())?;
            ComputePipeline::with_pipeline_layout(
                comp_queue.device().clone(),
                shader.entry_point("main").unwrap(),
                &spec_const,
                sim_pipeline_layout.clone(),
                None,
            )?
        };
        let slide_down_swap_pipeline = {
            let shader = slide_down_swap_cs::load(comp_queue.device().clone())?;
            ComputePipeline::with_pipeline_layout(
                comp_queue.device().clone(),
                shader.entry_point("main").unwrap(),
                &spec_const,
                sim_pipeline_layout.clone(),
                None,
            )?
        };
        let horizontal_empty_pipeline = {
            let shader = horizontal_empty_cs::load(comp_queue.device().clone())?;
            ComputePipeline::with_pipeline_layout(
                comp_queue.device().clone(),
                shader.entry_point("main").unwrap(),
                &spec_const,
                sim_pipeline_layout.clone(),
                None,
            )?
        };
        let horizontal_swap_pipeline = {
            let shader = horizontal_swap_cs::load(comp_queue.device().clone())?;
            ComputePipeline::with_pipeline_layout(
                comp_queue.device().clone(),
                shader.entry_point("main").unwrap(),
                &spec_const,
                sim_pipeline_layout.clone(),
                None,
            )?
        };
        let react_pipeline = {
            let shader = react_cs::load(comp_queue.device().clone())?;
            ComputePipeline::with_pipeline_layout(
                comp_queue.device().clone(),
                shader.entry_point("main").unwrap(),
                &spec_const,
                sim_pipeline_layout.clone(),
                None,
            )?
        };
        let color_pipeline = {
            let shader = color_cs::load(comp_queue.device().clone())?;
            ComputePipeline::with_pipeline_layout(
                comp_queue.device().clone(),
                shader.entry_point("main").unwrap(),
                &spec_const,
                sim_pipeline_layout,
                None,
            )?
        };
        let init_pipeline = {
            let shader = init_cs::load(comp_queue.device().clone())?;
            ComputePipeline::with_pipeline_layout(
                comp_queue.device().clone(),
                shader.entry_point("main").unwrap(),
                &spec_const,
                utils_pipeline_layout.clone(),
                None,
            )?
        };
        let finish_pipeline = {
            let shader = finish_cs::load(comp_queue.device().clone())?;
            ComputePipeline::with_pipeline_layout(
                comp_queue.device().clone(),
                shader.entry_point("main").unwrap(),
                &spec_const,
                utils_pipeline_layout.clone(),
                None,
            )?
        };
        let update_bitmap_pipeline = {
            let shader = update_bitmap_cs::load(comp_queue.device().clone())?;
            ComputePipeline::with_pipeline_layout(
                comp_queue.device().clone(),
                shader.entry_point("main").unwrap(),
                &spec_const,
                utils_pipeline_layout,
                None,
            )?
        };

        Ok(CASimulator {
            comp_queue,
            fall_empty_pipeline,
            fall_swap_pipeline,
            rise_empty_pipeline,
            rise_swap_pipeline,
            slide_down_empty_pipeline,
            slide_down_swap_pipeline,
            horizontal_empty_pipeline,
            horizontal_swap_pipeline,
            react_pipeline,
            color_pipeline,

            init_pipeline,
            update_bitmap_pipeline,
            finish_pipeline,

            matter_color_input,
            matter_state_input,
            matter_weight_input,
            matter_dispersion_input,
            matter_characteristics_input,
            matter_reaction_with_input,
            matter_reaction_direction_input,
            matter_reaction_probability_input,
            matter_reaction_transition_input,

            bitmap,

            tmp_matter,
            sim_steps: 0,
            dispersion_step: 0,
            dispersion_dir: 0,
            move_step: 0,
            sim_pos_offset: Vector2::new(0, 0),
            seed: 0.0,
            start: Instant::now(),
        })
    }

    pub(crate) fn update_matter_data(
        &mut self,
        matter_definitions: &MatterDefinitions,
    ) -> Result<()> {
        let mut write_matter_color_input = self.matter_color_input.write()?;
        let mut write_matter_state_input = self.matter_state_input.write()?;
        let mut write_matter_weight_input = self.matter_weight_input.write()?;
        let mut write_matter_dispersion_input = self.matter_dispersion_input.write()?;
        let mut write_matter_characteristics_input = self.matter_characteristics_input.write()?;
        let mut write_matter_reaction_with_input = self.matter_reaction_with_input.write()?;
        let mut write_matter_reaction_direction_input =
            self.matter_reaction_direction_input.write()?;
        let mut write_matter_reaction_probability_input =
            self.matter_reaction_probability_input.write()?;
        let mut write_matter_reaction_transition_input =
            self.matter_reaction_transition_input.write()?;
        let zero = MatterDefinition::zero();
        for i in 0..MAX_NUM_MATTERS as usize {
            let matter = if i < matter_definitions.definitions.len() {
                &matter_definitions.definitions[i]
            } else {
                &zero
            };
            write_matter_color_input[i] = u32_rgba_to_u32_abgr(matter.color);
            write_matter_state_input[i] = matter.state as u32;
            write_matter_weight_input[i] = matter.weight;
            write_matter_dispersion_input[i] = matter.dispersion;
            write_matter_characteristics_input[i] = matter.characteristics.bits();
            let table_index = i * MAX_TRANSITIONS as usize;
            for j in 0..(MAX_TRANSITIONS as usize) {
                write_matter_reaction_with_input[table_index + j] =
                    matter.reactions[j].reacts.bits();
                write_matter_reaction_direction_input[table_index + j] =
                    matter.reactions[j].direction.bits();
                write_matter_reaction_probability_input[table_index + j] =
                    matter.reactions[j].probability;
                write_matter_reaction_transition_input[table_index + j] =
                    matter.reactions[j].becomes as u32;
            }
        }
        Ok(())
    }

    pub fn update_bitmaps(
        &self,
        solid_bitmap: &mut [f64],
        powder_bitmap: &mut [f64],
        liquid_bitmap: &mut [f64],
        solids_changed: &mut bool,
        powders_changed: &mut bool,
        liquids_changed: &mut bool,
    ) -> Result<()> {
        let gpu_bitmap = self.bitmap.read()?;
        for i in 0..gpu_bitmap.len() {
            let gpu_val = gpu_bitmap[i];
            let old_solid = solid_bitmap[i];
            let old_powder = powder_bitmap[i];
            let old_liquid = liquid_bitmap[i];

            let new_solid = (gpu_val & (1 << 0)) as f64;
            let new_powder = (gpu_val & (1 << 1)) as f64;
            let new_liquid = (gpu_val & (1 << 2)) as f64;

            solid_bitmap[i] = new_solid;
            powder_bitmap[i] = new_powder;
            liquid_bitmap[i] = new_liquid;

            if !*solids_changed {
                *solids_changed = old_solid != new_solid;
            }
            if !*powders_changed {
                *powders_changed = old_powder != new_powder;
            }
            if !*liquids_changed {
                *liquids_changed = old_liquid != new_liquid;
            }
        }

        Ok(())
    }

    pub fn step(
        &mut self,
        settings: AppSettings,
        sim_pos_offset: Vector2<i32>,
        chunk_manager: &mut SimulationChunkManager,
    ) -> Result<()> {
        self.seed = (Instant::now() - self.start).as_secs_f32();
        // Get chunks for compute
        let mut world_chunks = chunk_manager.get_chunks_for_compute();
        // Run ca simulation
        self.sim_pos_offset = sim_pos_offset;
        let mut builder = AutoCommandBufferBuilder::primary(
            self.comp_queue.device().clone(),
            self.comp_queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )?;

        // Inits
        self.dispatch_utility(&mut builder, self.init_pipeline.clone(), &mut world_chunks)?;

        // Movement
        // ------
        self.move_once(&mut builder, 0, &mut world_chunks)?;
        self.disperse(
            &mut builder,
            (self.sim_steps % 2 == 0) as u32,
            &mut world_chunks,
            settings.dispersion_steps,
        )?;
        if settings.movement_steps > 1 {
            self.move_once(&mut builder, 1, &mut world_chunks)?;
        }
        if settings.movement_steps > 2 {
            self.move_once(&mut builder, 2, &mut world_chunks)?;
        }
        self.disperse(
            &mut builder,
            (self.sim_steps % 2 != 0) as u32,
            &mut world_chunks,
            settings.dispersion_steps,
        )?;
        // ------

        // React
        self.dispatch(
            &mut builder,
            self.react_pipeline.clone(),
            &mut world_chunks,
            true,
        )?;

        // Finish
        self.dispatch_utility(
            &mut builder,
            self.finish_pipeline.clone(),
            &mut world_chunks,
        )?;
        self.dispatch_utility(
            &mut builder,
            self.update_bitmap_pipeline.clone(),
            &mut world_chunks,
        )?;
        self.dispatch(
            &mut builder,
            self.color_pipeline.clone(),
            &mut world_chunks,
            false,
        )?;

        let command_buffer = builder.build()?;
        let finished = command_buffer.execute(self.comp_queue.clone())?;
        let _fut = finished.then_signal_fence_and_flush()?;
        self.sim_steps += 1;

        // Step flips matter grids, thus update mutated matter grids back to chunk manager after
        chunk_manager.update_compute_chunks(world_chunks.1);
        Ok(())
    }

    fn move_once(
        &mut self,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        step: u32,
        world_chunks: &mut (Vector2<i32>, Vec<GpuChunk>),
    ) -> Result<()> {
        self.move_step = step;
        // Anything that falls
        self.dispatch(
            builder,
            self.fall_empty_pipeline.clone(),
            world_chunks,
            true,
        )?;
        self.dispatch(builder, self.fall_swap_pipeline.clone(), world_chunks, true)?;
        // Risers
        self.dispatch(
            builder,
            self.rise_empty_pipeline.clone(),
            world_chunks,
            true,
        )?;
        self.dispatch(builder, self.rise_swap_pipeline.clone(), world_chunks, true)?;
        // Sliders
        self.dispatch(
            builder,
            self.slide_down_empty_pipeline.clone(),
            world_chunks,
            true,
        )?;
        self.dispatch(
            builder,
            self.slide_down_swap_pipeline.clone(),
            world_chunks,
            true,
        )?;
        Ok(())
    }

    fn disperse(
        &mut self,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        direction: u32,
        world_chunks: &mut (Vector2<i32>, Vec<GpuChunk>),
        dispersion_steps: u32,
    ) -> Result<()> {
        self.dispersion_dir = direction;
        for dispersion_step in 0..dispersion_steps {
            self.dispersion_step = dispersion_step as u32;
            self.dispatch(
                builder,
                self.horizontal_empty_pipeline.clone(),
                world_chunks,
                true,
            )?;
            self.dispatch(
                builder,
                self.horizontal_swap_pipeline.clone(),
                world_chunks,
                true,
            )?;
        }
        Ok(())
    }

    fn dispatch(
        &mut self,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        pipeline: Arc<ComputePipeline>,
        world_chunks: &mut (Vector2<i32>, Vec<GpuChunk>),
        swap: bool,
    ) -> Result<()> {
        let pipeline_layout = pipeline.layout();
        let desc_layout = pipeline_layout.descriptor_set_layouts().get(0).unwrap();
        let (chunk_start, chunks) = world_chunks;

        let set = PersistentDescriptorSet::new(desc_layout.clone(), [
            WriteDescriptorSet::buffer(0, self.matter_color_input.clone()),
            WriteDescriptorSet::buffer(1, self.matter_state_input.clone()),
            WriteDescriptorSet::buffer(2, self.matter_weight_input.clone()),
            WriteDescriptorSet::buffer(3, self.matter_dispersion_input.clone()),
            WriteDescriptorSet::buffer(4, self.matter_characteristics_input.clone()),
            WriteDescriptorSet::buffer(5, self.matter_reaction_with_input.clone()),
            WriteDescriptorSet::buffer(6, self.matter_reaction_direction_input.clone()),
            WriteDescriptorSet::buffer(7, self.matter_reaction_probability_input.clone()),
            WriteDescriptorSet::buffer(8, self.matter_reaction_transition_input.clone()),
            WriteDescriptorSet::buffer(9, chunks[0].matter_in.clone()),
            WriteDescriptorSet::buffer(10, chunks[0].matter_out.clone()),
            WriteDescriptorSet::buffer(11, chunks[0].objects_matter.clone()),
            WriteDescriptorSet::buffer(12, chunks[0].objects_color.clone()),
            WriteDescriptorSet::image_view(13, chunks[0].image.clone()),
            WriteDescriptorSet::buffer(14, chunks[1].matter_in.clone()),
            WriteDescriptorSet::buffer(15, chunks[1].matter_out.clone()),
            WriteDescriptorSet::buffer(16, chunks[1].objects_matter.clone()),
            WriteDescriptorSet::buffer(17, chunks[1].objects_color.clone()),
            WriteDescriptorSet::image_view(18, chunks[1].image.clone()),
            WriteDescriptorSet::buffer(19, chunks[2].matter_in.clone()),
            WriteDescriptorSet::buffer(20, chunks[2].matter_out.clone()),
            WriteDescriptorSet::buffer(21, chunks[2].objects_matter.clone()),
            WriteDescriptorSet::buffer(22, chunks[2].objects_color.clone()),
            WriteDescriptorSet::image_view(23, chunks[2].image.clone()),
            WriteDescriptorSet::buffer(24, chunks[3].matter_in.clone()),
            WriteDescriptorSet::buffer(25, chunks[3].matter_out.clone()),
            WriteDescriptorSet::buffer(26, chunks[3].objects_matter.clone()),
            WriteDescriptorSet::buffer(27, chunks[3].objects_color.clone()),
            WriteDescriptorSet::image_view(28, chunks[3].image.clone()),
        ])?;

        // Note that we make an assumption here that PCs are same for all our simulation kernel (see `shared.glsl`)
        // hence react_cs::...
        let push_constants = react_cs::ty::PushConstants {
            seed: self.seed,
            sim_step: self.sim_steps as u32,
            move_step: self.move_step as u32,
            dispersion_step: self.dispersion_step,
            dispersion_dir: self.dispersion_dir,
            sim_pos_offset: self.sim_pos_offset.into(),
            sim_chunk_start_offset: (*chunk_start).into(),
            _dummy0: [0; 4],
        };
        builder
            .bind_pipeline_compute(pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Compute, pipeline_layout.clone(), 0, set)
            .push_constants(pipeline_layout.clone(), 0, push_constants)
            .dispatch([
                *SIM_CANVAS_SIZE / KERNEL_SIZE,
                *SIM_CANVAS_SIZE / KERNEL_SIZE,
                1,
            ])?;
        if swap {
            for chunk in chunks.iter_mut() {
                // Swap matter in & out
                let temp = chunk.matter_out.clone();
                chunk.matter_out = chunk.matter_in.clone();
                chunk.matter_in = temp;
            }
        }

        Ok(())
    }

    /// Why this? Because macos doesn't allow > 30 buffer inputs
    fn dispatch_utility(
        &mut self,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        pipeline: Arc<ComputePipeline>,
        world_chunks: &mut (Vector2<i32>, Vec<GpuChunk>),
    ) -> Result<()> {
        let pipeline_layout = pipeline.layout();
        let desc_layout = pipeline_layout.descriptor_set_layouts().get(0).unwrap();
        let (chunk_start, chunks) = world_chunks;

        let set = PersistentDescriptorSet::new(desc_layout.clone(), [
            WriteDescriptorSet::buffer(0, self.matter_color_input.clone()),
            WriteDescriptorSet::buffer(1, self.matter_state_input.clone()),
            WriteDescriptorSet::buffer(2, self.bitmap.clone()),
            WriteDescriptorSet::buffer(3, chunks[0].matter_in.clone()),
            WriteDescriptorSet::buffer(4, chunks[0].matter_out.clone()),
            WriteDescriptorSet::buffer(5, chunks[0].objects_matter.clone()),
            WriteDescriptorSet::buffer(6, chunks[1].matter_in.clone()),
            WriteDescriptorSet::buffer(7, chunks[1].matter_out.clone()),
            WriteDescriptorSet::buffer(8, chunks[1].objects_matter.clone()),
            WriteDescriptorSet::buffer(9, chunks[2].matter_in.clone()),
            WriteDescriptorSet::buffer(10, chunks[2].matter_out.clone()),
            WriteDescriptorSet::buffer(11, chunks[2].objects_matter.clone()),
            WriteDescriptorSet::buffer(12, chunks[3].matter_in.clone()),
            WriteDescriptorSet::buffer(13, chunks[3].matter_out.clone()),
            WriteDescriptorSet::buffer(14, chunks[3].objects_matter.clone()),
            WriteDescriptorSet::buffer(15, self.tmp_matter.clone()),
        ])?;

        // Note that we make an assumption here that PCs are same for all our simulation kernel (see `shared.glsl`)
        let push_constants = init_cs::ty::PushConstants {
            sim_pos_offset: self.sim_pos_offset.into(),
            sim_chunk_start_offset: (*chunk_start).into(),
        };
        builder
            .bind_pipeline_compute(pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Compute, pipeline_layout.clone(), 0, set)
            .push_constants(pipeline_layout.clone(), 0, push_constants)
            .dispatch([
                *SIM_CANVAS_SIZE / KERNEL_SIZE,
                *SIM_CANVAS_SIZE / KERNEL_SIZE,
                1,
            ])?;

        Ok(())
    }
}

#[allow(deprecated)]
mod fall_empty_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute_shaders/simulation/fall_empty.glsl",
    }
}

#[allow(deprecated)]
mod fall_swap_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute_shaders/simulation/fall_swap.glsl",
    }
}

#[allow(deprecated)]
mod rise_empty_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute_shaders/simulation/rise_empty.glsl",
    }
}

#[allow(deprecated)]
mod rise_swap_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute_shaders/simulation/rise_swap.glsl",
    }
}

#[allow(deprecated)]
mod slide_down_empty_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute_shaders/simulation/slide_down_empty.glsl",
    }
}

#[allow(deprecated)]
mod slide_down_swap_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute_shaders/simulation/slide_down_swap.glsl",
    }
}

#[allow(deprecated)]
mod horizontal_empty_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute_shaders/simulation/horizontal_empty.glsl",
    }
}

#[allow(deprecated)]
mod horizontal_swap_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute_shaders/simulation/horizontal_swap.glsl",
    }
}

#[allow(deprecated)]
mod react_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute_shaders/simulation/react.glsl",
    }
}

#[allow(deprecated)]
mod color_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute_shaders/simulation/color.glsl",
    }
}

#[allow(deprecated)]
mod init_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute_shaders/utils/init.glsl",
    }
}

#[allow(deprecated)]
mod update_bitmap_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute_shaders/utils/update_bitmap.glsl",
    }
}

#[allow(deprecated)]
mod finish_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute_shaders/utils/finish.glsl",
    }
}
