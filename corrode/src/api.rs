use std::hash::Hash;

use anyhow::*;
use cgmath::Vector2;
use egui_winit_vulkano::Gui;
use hecs::{Entity, World};
use rapier2d::prelude::*;
use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::{
    input_system::{InputButton, InputSystem},
    physics::PhysicsWorld,
    renderer::{Camera2D, Renderer},
    time::TimeTracker,
};

/// Context through which you can access renderer, inputs, time etc.
pub struct EngineApi<I: Hash + Eq + Copy + 'static> {
    pub ecs_world: World,
    pub physics_world: PhysicsWorld,
    pub gui: Gui,
    pub renderer: Renderer,
    pub inputs: Vec<InputSystem<I>>,
    pub main_camera: Camera2D,
    pub time: TimeTracker,
    pub thread_pool: ThreadPool,
}

impl<I: Hash + Eq + Copy + 'static> EngineApi<I> {
    pub fn new(
        input_mappings: Vec<Vec<(I, InputButton)>>,
        renderer: Renderer,
    ) -> Result<EngineApi<I>> {
        let public_time = TimeTracker::new();
        let gui = Gui::new(renderer.surface(), renderer.graphics_queue(), true);
        let main_camera = Camera2D::default();
        let num_threads = num_cpus::get_physical();
        let thread_pool = ThreadPoolBuilder::new().num_threads(num_threads).build()?;

        // For each mapping vector, create an input system
        let input_systems = input_mappings
            .iter()
            .map(|mapping| Self::input_system(mapping))
            .collect::<Vec<InputSystem<I>>>();

        Ok(EngineApi {
            ecs_world: World::new(),
            physics_world: PhysicsWorld::new(),
            gui,
            renderer,
            inputs: input_systems,
            main_camera,
            time: public_time,
            thread_pool,
        })
    }

    pub fn reset_world(&mut self) -> Result<()> {
        self.ecs_world = World::new();
        self.physics_world = PhysicsWorld::new();
        Ok(())
    }

    /// Creates input system with input action of type T.
    /// Scale factor is needed for correct positions
    pub fn input_system(action_mapping: &[(I, InputButton)]) -> InputSystem<I> {
        let mut input = InputSystem::<I>::new();
        input.mapper_mut().set(action_mapping);
        input
    }
}

pub fn remove_physics_entity(
    ecs_world: &mut World,
    physics_world: &mut PhysicsWorld,
    entity: Entity,
) {
    let rb = if let std::result::Result::Ok(rb) = ecs_world.get::<RigidBodyHandle>(entity) {
        Some(*rb)
    } else {
        None
    };
    if let Some(rb) = rb {
        physics_world.remove_physics(rb);
    }
    ecs_world.despawn(entity).unwrap();
}

pub fn physics_entity_at_pos(
    physics_world: &PhysicsWorld,
    world_pos: Vector2<f32>,
) -> Option<(&RigidBody, Entity)> {
    if let Some(rb) = physics_world.rigid_body_at_pos(world_pos) {
        let user_data = rb.user_data as u64;
        let entity = Entity::from_bits(user_data).unwrap();
        Some((rb, entity))
    } else {
        None
    }
}

#[allow(unused)]
pub fn physics_entity_mut_at_pos(
    physics_world: &mut PhysicsWorld,
    world_pos: Vector2<f32>,
) -> Option<(&mut RigidBody, Entity)> {
    if let Some(rb) = physics_world.rigid_body_mut_at_pos(world_pos) {
        let user_data = rb.user_data as u64;
        let entity = Entity::from_bits(user_data).unwrap();
        Some((rb, entity))
    } else {
        None
    }
}
