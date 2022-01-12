use std::sync::Arc;

use anyhow::*;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    device::Device,
};

#[allow(unused)]
pub fn empty_f32(device: Arc<Device>, size: usize) -> Result<Arc<CpuAccessibleBuffer<[f32]>>> {
    Ok(CpuAccessibleBuffer::from_iter(
        device,
        BufferUsage::all(),
        false,
        vec![0.0; size].into_iter(),
    )?)
}

#[allow(unused)]
pub fn empty_u32(device: Arc<Device>, size: usize) -> Result<Arc<CpuAccessibleBuffer<[u32]>>> {
    Ok(CpuAccessibleBuffer::from_iter(
        device,
        BufferUsage::all(),
        false,
        vec![0; size].into_iter(),
    )?)
}
