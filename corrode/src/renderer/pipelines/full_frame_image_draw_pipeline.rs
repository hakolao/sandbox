use std::sync::Arc;

use anyhow::*;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
    command_buffer::SecondaryAutoCommandBuffer,
    descriptor_set::PersistentDescriptorSet,
    device::Queue,
    image::ImageViewAbstract,
    pipeline::{
        graphics::{
            color_blend::ColorBlendState,
            input_assembly::InputAssemblyState,
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

pub struct FullFrameImagePipeline {
    gfx_queue: Arc<Queue>,
    pipeline: Arc<GraphicsPipeline>,
    pipeline_alpha: Arc<GraphicsPipeline>,
    vertices: Arc<CpuAccessibleBuffer<[TextVertex]>>,
    indices: Arc<CpuAccessibleBuffer<[u32]>>,
}

impl FullFrameImagePipeline {
    pub fn new(gfx_queue: Arc<Queue>, subpass: Subpass) -> Result<FullFrameImagePipeline> {
        let (vertices, indices) = textured_quad([0.0; 4], 2.0, 2.0);
        let vertex_buffer = CpuAccessibleBuffer::<[TextVertex]>::from_iter(
            gfx_queue.device().clone(),
            BufferUsage::vertex_buffer(),
            false,
            vertices.into_iter(),
        )?;
        let index_buffer = CpuAccessibleBuffer::<[u32]>::from_iter(
            gfx_queue.device().clone(),
            BufferUsage::index_buffer(),
            false,
            indices.into_iter(),
        )?;

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
        Ok(FullFrameImagePipeline {
            gfx_queue,
            pipeline,
            pipeline_alpha,
            vertices: vertex_buffer,
            indices: index_buffer,
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
            SamplerAddressMode::Repeat,
        )
    }

    pub fn draw(
        &mut self,
        viewport_dimensions: [u32; 2],
        image: Arc<dyn ImageViewAbstract + 'static>,
        is_alpha: bool,
        invert_y: bool,
    ) -> Result<SecondaryAutoCommandBuffer> {
        let pipeline = if is_alpha {
            self.pipeline_alpha.clone()
        } else {
            self.pipeline.clone()
        };
        let mut builder =
            command_buffer_builder(self.gfx_queue.clone(), pipeline.subpass().clone())?;
        let desc_set = self.create_descriptor_set(image)?;
        let index_count = self.indices.len() as u32;
        let push_constants = vs::ty::PushConstants {
            invert_y: invert_y as i32,
        };
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
            .push_constants(pipeline.layout().clone(), 0, push_constants)
            .bind_vertex_buffers(0, self.vertices.clone())
            .bind_index_buffer(self.indices.clone())
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
        path: "shaders/frame_vert.glsl"
    }
}

#[allow(deprecated)]
mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/frame_frag.glsl"
    }
}
