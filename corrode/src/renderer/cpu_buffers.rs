use std::sync::Arc;

use anyhow::*;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    device::Device,
};

use crate::renderer::TextVertex;

/// Return CPU accessible buffers for textured vertex and its indices by given vertices and indices
pub fn textured_vertex_cpu_buffers_with_indices(
    device: &Arc<Device>,
    vertices: Vec<TextVertex>,
    indices: Vec<u32>,
    host_cached: bool,
) -> Result<(
    Arc<CpuAccessibleBuffer<[TextVertex]>>,
    Arc<CpuAccessibleBuffer<[u32]>>,
)> {
    let vert_buf = CpuAccessibleBuffer::<[TextVertex]>::from_iter(
        device.clone(),
        BufferUsage::vertex_buffer(),
        host_cached,
        vertices.into_iter(),
    )?;
    let indices_buf = CpuAccessibleBuffer::<[u32]>::from_iter(
        device.clone(),
        BufferUsage::index_buffer(),
        host_cached,
        indices.into_iter(),
    )?;
    Ok((vert_buf, indices_buf))
}
