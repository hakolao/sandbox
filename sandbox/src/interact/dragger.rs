use cgmath::Vector2;
use corrode::{
    api::{physics_entity_at_pos, EngineApi},
    physics::PhysicsWorld,
};
use hecs::{Entity, World};
use rapier2d::prelude::*;

use crate::{
    app::InputAction,
    object::{Angle, Position},
    utils::rotate_radians,
};

pub struct EditorDragger {
    /// (object id that is dragged, local position relative to obj center)
    pub dragged_object: Option<(Entity, Vector2<f32>)>,
}

impl EditorDragger {
    pub fn drag_point(&self, obj_pos: Vector2<f32>, obj_angle: f32) -> Option<Vector2<f32>> {
        if let Some((_o, pos)) = self.dragged_object {
            return Some(rotate_radians(pos, obj_angle) + obj_pos);
        }
        None
    }

    pub fn drag_object(
        &mut self,
        api: &mut EngineApi<InputAction>,
        dragged_obj_data: &(Entity, Vector2<f32>),
    ) {
        let EngineApi {
            ecs_world,
            physics_world,
            main_camera,
            inputs,
            ..
        } = api;
        let mouse_world_pos =
            main_camera.screen_to_world_pos(inputs[0].mouse_position_normalized());
        let obj_id = dragged_obj_data.0;
        if let Ok(rb) = ecs_world.get::<RigidBodyHandle>(obj_id) {
            let rigid_body = &mut physics_world.physics.bodies[*rb];
            let translation = rigid_body.position().translation;
            let current_pos = Vector2::new(translation.x, translation.y);
            if let Some(drag_pos) = self.drag_point(current_pos, rigid_body.rotation().angle()) {
                let offset_to_mouse = mouse_world_pos - drag_pos;
                let prev_lin_vel = rigid_body.linvel().xy();
                let k = 30.0;
                let b = 1.5;
                let drag_force = vector![
                    k * offset_to_mouse.x - b * prev_lin_vel.x,
                    k * offset_to_mouse.y - b * prev_lin_vel.y
                ];
                rigid_body.add_force_at_point(drag_force, point![drag_pos.x, drag_pos.y], true);
                // Damp angular velocity
                let angvel = rigid_body.angvel();
                rigid_body.set_angvel(angvel * 0.95, false);
            }
        }
    }

    pub fn set_dragged_object(
        &mut self,
        ecs_world: &World,
        physics_world: &PhysicsWorld,
        mouse_world_pos: Vector2<f32>,
    ) {
        self.dragged_object = physics_entity_at_pos(physics_world, mouse_world_pos).and_then(|o| {
            if o.0.is_dynamic() {
                let pos = *ecs_world.get::<Position>(o.1).unwrap();
                let angle = *ecs_world.get::<Angle>(o.1).unwrap();
                let diff = mouse_world_pos - pos.0;
                Some((o.1, rotate_radians(diff, -angle.0)))
            } else {
                None
            }
        });
    }
}
