use anyhow::*;
use cgmath::Vector2;

use crate::sim::Simulation;

pub struct EditorPainter {
    pub matter: u32,
    pub radius: f32,
    pub is_square: bool,
}

impl EditorPainter {
    pub fn paint_round_line(
        &mut self,
        simulation: &mut Simulation,
        line: &[Vector2<i32>],
    ) -> Result<()> {
        simulation.paint_round(line, self.matter, self.radius)
    }

    pub fn paint_square_line(
        &mut self,
        simulation: &mut Simulation,
        line: &[Vector2<i32>],
    ) -> Result<()> {
        simulation.paint_square(line, self.matter, (self.radius * 2.0) as i32)
    }
}
