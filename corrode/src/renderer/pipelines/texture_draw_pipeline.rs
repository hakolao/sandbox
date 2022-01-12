use std::sync::Arc;

use anyhow::*;
use cgmath::{Matrix2, Vector2};
use vulkano::{
    buffer::{BufferAccess, BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
    command_buffer::SecondaryAutoCommandBuffer,
    descriptor_set::PersistentDescriptorSet,
    device::Queue,
    image::ImageViewAbstract,
    pipeline::{
        graphics::{
            color_blend::ColorBlendState,
            input_assembly::{Index, InputAssemblyState},
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::Subpass,
    sampler::SamplerAddressMode,
};

use crate::renderer::{
    pipelines::{command_buffer_builder, sampled_image_desc_set},
    textured_quad, TextVertex,
};

pub struct TextureDrawPipeline {
    gfx_queue: Arc<Queue>,
    pipeline: Arc<GraphicsPipeline>,
    pipeline_alpha: Arc<GraphicsPipeline>,
    vertices: Arc<CpuAccessibleBuffer<[TextVertex]>>,
    indices: Arc<CpuAccessibleBuffer<[u32]>>,
}

impl TextureDrawPipeline {
    pub fn new(gfx_queue: Arc<Queue>, subpass: Subpass) -> Result<TextureDrawPipeline> {
        let pipeline = {
            let vs = vs::load(gfx_queue.device().clone()).expect("failed to create shader module");
            let fs = fs::load(gfx_queue.device().clone()).expect("failed to create shader module");

            GraphicsPipeline::start()
                .vertex_input_state(BuffersDefinition::new().vertex::<TextVertex>())
                .vertex_shader(vs.entry_point("main").unwrap(), ())
                .fragment_shader(fs.entry_point("main").unwrap(), ())
                .input_assembly_state(InputAssemblyState::new())
                .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
                .render_pass(subpass.clone())
                .build(gfx_queue.device().clone())?
        };
        let pipeline_alpha = {
            let vs = vs::load(gfx_queue.device().clone()).expect("failed to create shader module");
            let fs = fs::load(gfx_queue.device().clone()).expect("failed to create shader module");

            GraphicsPipeline::start()
                .vertex_input_state(BuffersDefinition::new().vertex::<TextVertex>())
                .vertex_shader(vs.entry_point("main").unwrap(), ())
                .fragment_shader(fs.entry_point("main").unwrap(), ())
                .input_assembly_state(InputAssemblyState::new())
                .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
                .color_blend_state(ColorBlendState::new(1).blend_alpha())
                .render_pass(subpass)
                .build(gfx_queue.device().clone())?
        };
        let (v, i) = textured_quad([0.0; 4], 2.0, 2.0);
        let vertices = CpuAccessibleBuffer::from_iter(
            gfx_queue.device().clone(),
            BufferUsage::vertex_buffer(),
            false,
            v.into_iter(),
        )?;
        let indices = CpuAccessibleBuffer::from_iter(
            gfx_queue.device().clone(),
            BufferUsage::index_buffer(),
            false,
            i.into_iter(),
        )?;
        Ok(TextureDrawPipeline {
            gfx_queue,
            pipeline,
            pipeline_alpha,
            vertices,
            indices,
        })
    }

    fn create_descriptor_set(
        &self,
        image: Arc<dyn ImageViewAbstract + 'static>,
    ) -> Result<Arc<PersistentDescriptorSet>> {
        let layout = self
            .pipeline
            .layout()
            .descriptor_set_layouts()
            .get(0)
            .unwrap();
        sampled_image_desc_set(
            self.gfx_queue.clone(),
            layout,
            image,
            SamplerAddressMode::ClampToEdge,
        )
    }

    pub fn draw_texture_on_quad(
        &mut self,
        viewport_dimensions: [u32; 2],
        world_to_screen: cgmath::Matrix4<f32>,
        pos: Vector2<f32>,
        width: f32,
        height: f32,
        rotation: Matrix2<f32>,
        image: Arc<dyn ImageViewAbstract + 'static>,
        invert_y: bool,
        alpha: bool,
    ) -> Result<SecondaryAutoCommandBuffer> {
        let push_constants = vs::ty::PushConstants {
            world_to_screen: world_to_screen.into(),
            world_pos: pos.into(),
            rotation: rotation.into(),
            dims: [width, height],
            invert_y: invert_y as i32,
        };
        let mut builder =
            command_buffer_builder(self.gfx_queue.clone(), self.pipeline.subpass().clone())?;
        let desc_set = self.create_descriptor_set(image)?;
        let pipeline = if alpha {
            self.pipeline_alpha.clone()
        } else {
            self.pipeline.clone()
        };
        let index_count = self.indices.len() as u32;
        builder
            .bind_pipeline_graphics(pipeline.clone())
            .set_viewport(0, vec![Viewport {
                origin: [0.0, 0.0],
                dimensions: [viewport_dimensions[0] as f32, viewport_dimensions[1] as f32],
                depth_range: 0.0..1.0,
            }])
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                pipeline.layout().clone(),
                0,
                desc_set,
            )
            .bind_vertex_buffers(0, self.vertices.clone())
            .bind_index_buffer(self.indices.clone())
            .push_constants(pipeline.layout().clone(), 0, push_constants)
            .draw_indexed(index_count, 1, 0, 0, 0)
            .unwrap();
        let command_buffer = builder.build()?;
        Ok(command_buffer)
    }

    pub fn draw_mesh<V, Vb, Ib, I>(
        &mut self,
        viewport_dimensions: [u32; 2],
        world_to_screen: cgmath::Matrix4<f32>,
        pos: Vector2<f32>,
        rotation: Matrix2<f32>,
        image: Arc<dyn ImageViewAbstract + 'static>,
        vertices: Arc<Vb>,
        indices: Arc<Ib>,
        alpha: bool,
    ) -> Result<SecondaryAutoCommandBuffer>
    where
        Vb: BufferAccess + TypedBufferAccess<Content = [V]> + Send + Sync + 'static,
        Ib: BufferAccess + TypedBufferAccess<Content = [I]> + Send + Sync + 'static,
        I: Index + 'static,
    {
        let push_constants = vs::ty::PushConstants {
            world_to_screen: world_to_screen.into(),
            world_pos: pos.into(),
            rotation: rotation.into(),
            dims: [1.0, 1.0],
            invert_y: 0,
        };
        let mut builder =
            command_buffer_builder(self.gfx_queue.clone(), self.pipeline.subpass().clone())?;
        let desc_set = self.create_descriptor_set(image)?;
        let pipeline = if alpha {
            self.pipeline_alpha.clone()
        } else {
            self.pipeline.clone()
        };
        let index_count = indices.len() as u32;
        builder
            .bind_pipeline_graphics(pipeline.clone())
            .set_viewport(0, vec![Viewport {
                origin: [0.0, 0.0],
                dimensions: [viewport_dimensions[0] as f32, viewport_dimensions[1] as f32],
                depth_range: 0.0..1.0,
            }])
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                pipeline.layout().clone(),
                0,
                desc_set,
            )
            .bind_vertex_buffers(0, vertices)
            .bind_index_buffer(indices)
            .push_constants(pipeline.layout().clone(), 0, push_constants)
            .draw_indexed(index_count, 1, 0, 0, 0)
            .unwrap();
        let command_buffer = builder.build()?;
        Ok(command_buffer)
    }
}

#[allow(deprecated)]
mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/image_vert.glsl"
    }
}

#[allow(deprecated)]
mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/image_frag.glsl"
    }
}
