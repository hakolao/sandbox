use anyhow::*;
use cgmath::Vector2;
use corrode::{
    physics::PhysicsWorld,
    renderer::{render_pass::DrawPass, Line},
};
use hecs::{Entity, World};
use rapier2d::prelude::*;

use crate::{
    object::PixelData,
    sim::{chunk_lines, get_collider_lines, Simulation},
    HALF_CELL, SIM_CANVAS_SIZE, WORLD_UNIT_SIZE,
};

fn get_boundary_contour_lines(
    ecs_world: &World,
    physics_world: &PhysicsWorld,
    boundary_entities: &[Entity],
    color: [f32; 4],
) -> Vec<Line> {
    let mut lines = vec![];
    for e in boundary_entities.iter() {
        let rb = *ecs_world.get::<RigidBodyHandle>(*e).unwrap();
        let rigid_body = &physics_world.physics.bodies[rb];
        for c in rigid_body.colliders() {
            let collider = &physics_world.physics.colliders[*c];
            if collider.shape().as_polyline().is_some() {
                lines.extend(get_collider_lines(collider, color));
            }
        }
    }
    lines
}

pub fn draw_canvas(simulation: &Simulation, draw_pass: &mut DrawPass) -> Result<()> {
    for chunk in simulation.chunk_manager.get_chunks_for_render() {
        let chunk_pos =
            Vector2::new(chunk.0.x as f32, chunk.0.y as f32) * WORLD_UNIT_SIZE - *HALF_CELL;
        let chunk_image = chunk.1.image.clone();
        draw_pass.draw_texture(
            chunk_pos,
            WORLD_UNIT_SIZE / 2.0,
            WORLD_UNIT_SIZE / 2.0,
            0.0,
            chunk_image,
            true,
            false,
        )?
    }
    Ok(())
}

pub fn draw_contours(
    ecs_world: &World,
    physics_world: &PhysicsWorld,
    simulation: &Simulation,
    draw_pass: &mut DrawPass,
) -> Result<()> {
    let mut lines = vec![];
    // Pixel Objects
    for (_id, (rb, ..)) in &mut ecs_world.query::<(&RigidBodyHandle, &PixelData)>() {
        let rigid_body = &physics_world.physics.bodies[*rb];
        for c in rigid_body.colliders() {
            let collider = &physics_world.physics.colliders[*c];
            if collider.shape().as_compound().is_some() {
                lines.extend(get_collider_lines(collider, [1.0, 0.0, 0.0, 1.0]));
            }
        }
    }
    // Polylines (utils)
    lines.extend(get_boundary_contour_lines(
        ecs_world,
        physics_world,
        &simulation.boundaries.solid_objects,
        [0.0, 1.0, 0.0, 1.0],
    ));
    lines.extend(get_boundary_contour_lines(
        ecs_world,
        physics_world,
        &simulation.boundaries.powder_objects,
        [1.0, 1.0, 0.0, 1.0],
    ));
    lines.extend(get_boundary_contour_lines(
        ecs_world,
        physics_world,
        &simulation.boundaries.liquid_objects,
        [0.0, 0.0, 1.0, 1.0],
    ));
    draw_pass.draw_lines(&lines)?;
    Ok(())
}

pub fn draw_grid(
    simulation: &Simulation,
    draw_pass: &mut DrawPass,
    grid_color: [f32; 4],
) -> Result<()> {
    let mut lines = vec![];
    let length = 20;
    let half_length = length / 2;
    let cam_chunk = simulation.camera_canvas_pos / *SIM_CANVAS_SIZE as i32;
    for y in -half_length..=half_length {
        for x in -half_length..=half_length {
            let chunk = Vector2::new(x, y) + cam_chunk;
            lines.extend(chunk_lines(chunk, grid_color));
        }
    }
    draw_pass.draw_lines(&lines)?;
    Ok(())
}

pub fn draw_debug_bounds(
    simulation: &Simulation,
    draw_pass: &mut DrawPass,
    sim_color: [f32; 4],
) -> Result<()> {
    let mut lines = vec![];
    lines.extend([
        Line(
            0.5 * Vector2::new(-WORLD_UNIT_SIZE, WORLD_UNIT_SIZE) + simulation.camera_pos
                - *HALF_CELL,
            0.5 * Vector2::new(WORLD_UNIT_SIZE, WORLD_UNIT_SIZE) + simulation.camera_pos
                - *HALF_CELL,
            sim_color,
        ),
        Line(
            0.5 * Vector2::new(-WORLD_UNIT_SIZE, -WORLD_UNIT_SIZE) + simulation.camera_pos
                - *HALF_CELL,
            0.5 * Vector2::new(WORLD_UNIT_SIZE, -WORLD_UNIT_SIZE) + simulation.camera_pos
                - *HALF_CELL,
            sim_color,
        ),
        Line(
            0.5 * Vector2::new(-WORLD_UNIT_SIZE, -WORLD_UNIT_SIZE) + simulation.camera_pos
                - *HALF_CELL,
            0.5 * Vector2::new(-WORLD_UNIT_SIZE, WORLD_UNIT_SIZE) + simulation.camera_pos
                - *HALF_CELL,
            sim_color,
        ),
        Line(
            0.5 * Vector2::new(WORLD_UNIT_SIZE, -WORLD_UNIT_SIZE) + simulation.camera_pos
                - *HALF_CELL,
            0.5 * Vector2::new(WORLD_UNIT_SIZE, WORLD_UNIT_SIZE) + simulation.camera_pos
                - *HALF_CELL,
            sim_color,
        ),
    ]);
    draw_pass.draw_lines(&lines)?;
    Ok(())
}

pub fn draw_chunk_debug_info(
    simulation: &Simulation,
    draw_pass: &mut DrawPass,
    chunk_color: [f32; 4],
    interaction_color: [f32; 4],
) -> Result<()> {
    let mut lines = vec![];
    for chunk in simulation.chunk_manager.chunks_in_use.iter() {
        lines.extend(chunk_lines(*chunk, chunk_color));
    }
    for chunk in simulation.chunk_manager.interaction_chunks.iter() {
        lines.extend(chunk_lines(*chunk, interaction_color));
    }
    draw_pass.draw_lines(&lines)?;
    Ok(())
}
