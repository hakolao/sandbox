use cgmath::Vector2;

/// A B C D
pub const VERTICES_PER_QUAD: usize = 4;
/// 0, 2, 1, 0, 3, 2
pub const INDICES_PER_QUAD: usize = 6;

/// 0, 1, 1, 2, 2, 3, 3, 4 (Line list)
pub const VERTICES_PER_LINE: usize = 2;

/// Vertex for textured quads
#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
pub struct TextVertex {
    pub position: [f32; 2],
    pub normal: [f32; 2],
    pub tex_coords: [f32; 2],
    pub color: [f32; 4],
}
vulkano::impl_vertex!(TextVertex, position, normal, tex_coords, color);

impl TextVertex {
    pub fn x(&self) -> f32 {
        self.position[0]
    }

    pub fn y(&self) -> f32 {
        self.position[1]
    }

    pub fn empty() -> TextVertex {
        TextVertex {
            position: [0.0; 2],
            normal: [0.0; 2],
            tex_coords: [0.0; 2],
            color: [0.0; 4],
        }
    }
}

pub fn textured_quad(color: [f32; 4], width: f32, height: f32) -> (Vec<TextVertex>, Vec<u32>) {
    (
        vec![
            TextVertex {
                position: [-(width / 2.0), -(height / 2.0)],
                normal: [0.0, 0.0],
                tex_coords: [0.0, 1.0],
                color,
            },
            TextVertex {
                position: [-(width / 2.0), height / 2.0],
                normal: [0.0, 0.0],
                tex_coords: [0.0, 0.0],
                color,
            },
            TextVertex {
                position: [width / 2.0, height / 2.0],
                normal: [0.0, 0.0],
                tex_coords: [1.0, 0.0],
                color,
            },
            TextVertex {
                position: [width / 2.0, -(height / 2.0)],
                normal: [0.0, 0.0],
                tex_coords: [1.0, 1.0],
                color,
            },
        ],
        vec![0, 2, 1, 0, 3, 2],
    )
}

pub struct Line(pub Vector2<f32>, pub Vector2<f32>, pub [f32; 4]);

pub fn line_vertices(lines: &[Line]) -> (Vec<TextVertex>, Vec<u32>) {
    let mut vertices = Vec::<TextVertex>::with_capacity(lines.len());
    let mut indices = Vec::<u32>::with_capacity(lines.len());
    let mut i = 0;
    for line in lines {
        vertices.push(TextVertex {
            position: [line.0.x, line.0.y],
            normal: [0.0, 0.0],
            tex_coords: [0.0, 0.0],
            color: line.2,
        });
        vertices.push(TextVertex {
            position: [line.1.x, line.1.y],
            normal: [0.0, 0.0],
            tex_coords: [0.0, 0.0],
            color: line.2,
        });
        indices.push(i);
        indices.push(i + 1);
        i += 2;
    }
    (vertices, indices)
}
