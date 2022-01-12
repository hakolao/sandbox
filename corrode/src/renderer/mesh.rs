use std::{
    fmt::{Debug, Formatter},
    sync::Arc,
};

use anyhow::*;
use cgmath::Vector2;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
    device::Device,
};

use crate::renderer::{textured_quad, TextVertex};

#[allow(unused)]
#[derive(Clone)]
pub struct Mesh {
    pub vertices: Arc<CpuAccessibleBuffer<[TextVertex]>>,
    pub indices: Arc<CpuAccessibleBuffer<[u32]>>,
}

impl Debug for Mesh {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mesh")
            .field("vertices len: {}", &self.vertices.len())
            .field("indices len: {}", &self.indices.len())
            .finish()
    }
}

impl Mesh {
    pub fn new(device: Arc<Device>, vertices: Vec<TextVertex>, indices: Vec<u32>) -> Result<Mesh> {
        let v = CpuAccessibleBuffer::from_iter(
            device.clone(),
            BufferUsage::vertex_buffer(),
            false,
            vertices.into_iter(),
        )?;
        let i = CpuAccessibleBuffer::from_iter(
            device,
            BufferUsage::index_buffer(),
            false,
            indices.into_iter(),
        )?;
        Ok(Mesh {
            vertices: v,
            indices: i,
        })
    }

    pub fn new_rect(device: Arc<Device>, width: f32, height: f32, color: [f32; 4]) -> Result<Mesh> {
        let (vertices, indices) = textured_quad(color, width, height);
        let v = CpuAccessibleBuffer::from_iter(
            device.clone(),
            BufferUsage::vertex_buffer(),
            false,
            vertices.into_iter(),
        )?;
        let i = CpuAccessibleBuffer::from_iter(
            device,
            BufferUsage::index_buffer(),
            false,
            indices.into_iter(),
        )?;
        Ok(Mesh {
            vertices: v,
            indices: i,
        })
    }

    pub fn vertices_and_indices(&self) -> Result<(Vec<Vector2<f32>>, Vec<[u32; 3]>)> {
        let vtcs = self.vertices.read()?;
        let idxs = self.indices.read()?;
        let mut vertices = vec![];
        let mut indices = vec![];
        for v in vtcs.iter() {
            vertices.push(Vector2::new(v.position[0], v.position[1]));
        }
        for i in 0..(idxs.len() / 3) {
            indices.push([idxs[i * 3], idxs[i * 3 + 1], idxs[i * 3 + 2]]);
        }
        Ok((vertices, indices))
    }
}
