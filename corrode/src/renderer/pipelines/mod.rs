use std::sync::Arc;

use anyhow::*;
pub use basic_draw_pipeline::*;
pub use circle_draw_pipeline::*;
pub use full_frame_image_draw_pipeline::*;
pub use line_draw_pipeline::*;
pub use texture_draw_pipeline::*;
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, SecondaryAutoCommandBuffer},
    descriptor_set::{layout::DescriptorSetLayout, PersistentDescriptorSet, WriteDescriptorSet},
    device::Queue,
    image::ImageViewAbstract,
    render_pass::Subpass,
    sampler::{Filter, Sampler, SamplerAddressMode, SamplerMipmapMode},
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

/// Creates a descriptor set for images with nearest mipmap mode (pixel perfect)
#[allow(unused)]
pub fn sampled_image_desc_set(
    gfx_queue: Arc<Queue>,
    layout: &Arc<DescriptorSetLayout>,
    image: Arc<dyn ImageViewAbstract + 'static>,
    sampler_mode: SamplerAddressMode,
) -> Result<Arc<PersistentDescriptorSet>> {
    let sampler_builder = Sampler::start(gfx_queue.device().clone())
        .filter(Filter::Nearest)
        .address_mode(sampler_mode)
        .mipmap_mode(SamplerMipmapMode::Nearest)
        .mip_lod_bias(0.0)
        .lod(0.0..=0.0);
    let sampler = sampler_builder.build()?;
    Ok(PersistentDescriptorSet::new(layout.clone(), [
        WriteDescriptorSet::image_view_sampler(0, image.clone(), sampler),
    ])?)
}
