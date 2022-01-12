use std::sync::Arc;

use anyhow::*;
use cgmath::Vector2;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
    command_buffer::SecondaryAutoCommandBuffer,
    device::Queue,
    pipeline::{
        graphics::{
            color_blend::ColorBlendState,
            input_assembly::InputAssemblyState,
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline, Pipeline,
    },
    render_pass::Subpass,
};

use crate::renderer::{pipelines::command_buffer_builder, textured_quad, TextVertex};

pub struct CircleDrawPipeline {
    gfx_queue: Arc<Queue>,
    pipeline: Arc<GraphicsPipeline>,
    vertices: Arc<CpuAccessibleBuffer<[TextVertex]>>,
    indices: Arc<CpuAccessibleBuffer<[u32]>>,
}

impl CircleDrawPipeline {
    pub fn new(gfx_queue: Arc<Queue>, subpass: Subpass) -> Result<CircleDrawPipeline> {
        let pipeline = {
            let vs =
                vs::load(gfx_queue.device().clone()).context("failed to create shader module")?;
            let fs =
                fs::load(gfx_queue.device().clone()).context("failed to create shader module")?;

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
        Ok(CircleDrawPipeline {
            gfx_queue,
            pipeline,
            vertices,
            indices,
        })
    }

    pub fn draw(
        &mut self,
        viewport_dimensions: [u32; 2],
        world_to_screen: cgmath::Matrix4<f32>,
        pos: Vector2<f32>,
        radius: f32,
        color: [f32; 4],
    ) -> Result<SecondaryAutoCommandBuffer> {
        let push_constants = vs::ty::PushConstants {
            world_to_screen: world_to_screen.into(),
            color,
            world_pos: pos.into(),
            radius,
        };
        let mut builder =
            command_buffer_builder(self.gfx_queue.clone(), self.pipeline.subpass().clone())?;
        let index_count = self.indices.len() as u32;
        builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .set_viewport(0, vec![Viewport {
                origin: [0.0, 0.0],
                dimensions: [viewport_dimensions[0] as f32, viewport_dimensions[1] as f32],
                depth_range: 0.0..1.0,
            }])
            .bind_vertex_buffers(0, self.vertices.clone())
            .bind_index_buffer(self.indices.clone())
            .push_constants(self.pipeline.layout().clone(), 0, push_constants)
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
        path: "shaders/circle_vert.glsl"
    }
}

#[allow(deprecated)]
mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/circle_frag.glsl"
    }
}
