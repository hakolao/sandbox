use std::sync::Arc;

use anyhow::*;
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, SubpassContents},
    device::Queue,
    format::Format,
    image::ImageAccess,
    render_pass::{Framebuffer, RenderPass, Subpass},
    sync::GpuFuture,
};

use crate::renderer::{pipelines::FullFrameImagePipeline, DeviceImageView, FinalImageView};

pub struct RenderPassPlaceOverFrame {
    gfx_queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    full_frame_image_pipeline: FullFrameImagePipeline,
}

impl RenderPassPlaceOverFrame {
    pub fn new(gfx_queue: Arc<Queue>, output_format: Format) -> Result<RenderPassPlaceOverFrame> {
        let render_pass = vulkano::single_pass_renderpass!(gfx_queue.device().clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: output_format,
                    samples: 1,
                }
            },
            pass: {
                    color: [color],
                    depth_stencil: {}
            }
        )?;
        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();
        let full_frame_image_pipeline = FullFrameImagePipeline::new(gfx_queue.clone(), subpass)?;
        Ok(RenderPassPlaceOverFrame {
            gfx_queue,
            render_pass,
            full_frame_image_pipeline,
        })
    }

    /// Place view exactly over swapchain image target.
    /// Texture draw pipeline uses a quad onto which it places the view.
    pub fn render<F>(
        &mut self,
        before_future: F,
        view: DeviceImageView,
        target: FinalImageView,
        is_alpha: bool,
        invert_y: bool,
    ) -> Result<Box<dyn GpuFuture>>
    where
        F: GpuFuture + 'static,
    {
        // Get dimensions
        let img_dims = target.image().dimensions().width_height();
        // Create framebuffer (must be in same order as render pass description in `new`
        let framebuffer = Framebuffer::start(self.render_pass.clone())
            .add(target)?
            .build()?;
        // Create primary command buffer builder
        let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
            self.gfx_queue.device().clone(),
            self.gfx_queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )?;
        // Begin render pass
        command_buffer_builder.begin_render_pass(
            framebuffer,
            SubpassContents::SecondaryCommandBuffers,
            vec![[0.0; 4].into()],
        )?;
        // Create secondary command buffer from texture pipeline & send draw commands
        let cb = self
            .full_frame_image_pipeline
            .draw(img_dims, view, is_alpha, invert_y)?;
        // Execute above commands (subpass)
        command_buffer_builder.execute_commands(cb)?;
        // End render pass
        command_buffer_builder.end_render_pass()?;
        // Build command buffer
        let command_buffer = command_buffer_builder.build()?;
        // Execute primary command buffer
        let after_future = before_future
            .then_execute(self.gfx_queue.clone(), command_buffer)?
            .then_signal_fence_and_flush()?;

        Ok(after_future.boxed())
    }
}
