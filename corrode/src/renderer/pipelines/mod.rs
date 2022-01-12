use std::sync::Arc;

use anyhow::*;
pub use basic_draw_pipeline::*;
pub use circle_draw_pipeline::*;
pub use full_frame_image_draw_pipeline::*;
pub use line_draw_pipeline::*;
pub use texture_draw_pipeline::*;
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, SecondaryAutoCommandBuffer},
    descriptor_set::{layout::DescriptorSetLayout, PersistentDescriptorSet},
    device::Queue,
    image::ImageViewAbstract,
    render_pass::Subpass,
    sampler::{Filter, MipmapMode, Sampler, SamplerAddressMode},
};
pub use wireframe_draw_pipeline::*;

mod basic_draw_pipeline;
mod circle_draw_pipeline;
mod full_frame_image_draw_pipeline;
mod line_draw_pipeline;
mod texture_draw_pipeline;
mod wireframe_draw_pipeline;

pub fn command_buffer_builder(
    gfx_queue: Arc<Queue>,
    subpass: Subpass,
) -> Result<AutoCommandBufferBuilder<SecondaryAutoCommandBuffer>> {
    let builder = AutoCommandBufferBuilder::secondary_graphics(
        gfx_queue.device().clone(),
        gfx_queue.family(),
        CommandBufferUsage::MultipleSubmit,
        subpass,
    )?;
    Ok(builder)
}

/// Creates a descriptor set for images
pub fn sampled_image_desc_set(
    gfx_queue: Arc<Queue>,
    layout: &Arc<DescriptorSetLayout>,
    image: Arc<dyn ImageViewAbstract + 'static>,
    sampler_mode: SamplerAddressMode,
) -> Result<Arc<PersistentDescriptorSet>> {
    let sampler = Sampler::new(
        gfx_queue.device().clone(),
        Filter::Nearest,
        Filter::Nearest,
        MipmapMode::Nearest,
        sampler_mode,
        sampler_mode,
        sampler_mode,
        0.0,
        1.0,
        0.0,
        0.0,
    )?;
    let mut builder = PersistentDescriptorSet::start(layout.clone());
    builder.add_sampled_image(image.clone(), sampler)?;
    let set = builder.build()?;
    Ok(set)
}
