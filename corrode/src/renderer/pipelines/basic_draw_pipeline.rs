use std::sync::Arc;

use anyhow::*;
use cgmath::{Matrix2, Vector2};
use vulkano::{
    buffer::{BufferAccess, TypedBufferAccess},
    command_buffer::SecondaryAutoCommandBuffer,
    device::Queue,
    pipeline::{
        graphics::{
            input_assembly::{Index, InputAssemblyState},
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline, Pipeline,
    },
    render_pass::Subpass,
};

use crate::renderer::{pipelines::command_buffer_builder, TextVertex};

pub struct BasicDrawPipeline {
    gfx_queue: Arc<Queue>,
    pipeline: Arc<GraphicsPipeline>,
}

impl BasicDrawPipeline {
    pub fn new(gfx_queue: Arc<Queue>, subpass: Subpass) -> Result<BasicDrawPipeline> {
        let pipeline = {
            let vs = vs::load(gfx_queue.device().clone()).expect("failed to create shader module");
            let fs = fs::load(gfx_queue.device().clone()).expect("failed to create shader module");

            GraphicsPipeline::start()
                .vertex_input_state(BuffersDefinition::new().vertex::<TextVertex>())
                .vertex_shader(vs.entry_point("main").unwrap(), ())
                .fragment_shader(fs.entry_point("main").unwrap(), ())
                .input_assembly_state(InputAssemblyState::new())
                .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
                .render_pass(subpass)
                .build(gfx_queue.device().clone())?
        };
        Ok(BasicDrawPipeline {
            gfx_queue,
            pipeline,
        })
    }

    pub fn draw_mesh<V, Vb, Ib, I>(
        &mut self,
        viewport_dimensions: [u32; 2],
        world_to_screen: cgmath::Matrix4<f32>,
        pos: Vector2<f32>,
        rotation: Matrix2<f32>,
        vertices: Arc<Vb>,
        indices: Arc<Ib>,
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
            forced_color: [0.0; 4],
            force_color: 0,
            _dummy0: [0u8; 8],
        };
        let mut builder =
            command_buffer_builder(self.gfx_queue.clone(), self.pipeline.subpass().clone())?;
        let index_count = indices.len() as u32;
        builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .set_viewport(0, vec![Viewport {
                origin: [0.0, 0.0],
                dimensions: [viewport_dimensions[0] as f32, viewport_dimensions[1] as f32],
                depth_range: 0.0..1.0,
            }])
            .bind_vertex_buffers(0, vertices)
            .bind_index_buffer(indices)
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
        path: "shaders/basic_vert.glsl"
    }
}

#[allow(deprecated)]
mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/basic_frag.glsl"
    }
}
