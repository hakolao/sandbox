use std::sync::Arc;

use anyhow::*;
use cgmath::Vector2;
use corrode::physics::{Physics, PhysicsWorld};
use hecs::{Entity, World};
use rapier2d::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    object::{
        Angle, AngularVelocity, DynamicRigidbody, LinearVelocity, MatterPixel, PixelData, Position,
        SensorRigidbody, StaticRigidbody, TempPixel,
    },
    simulation::Simulation,
    utils::BitmapImage,
};

/// Data needed to create new objects after deformation. Vec<f64> represents the bitmap which is used
/// to create new pixel data from the previous object's PixelData.
pub type DeformedObjectData = (
    Entity,
    RigidBodyHandle,
    PixelData,
    Position,
    LinearVelocity,
    Angle,
    AngularVelocity,
    Vec<f64>,
);

/// Data needed to create a dynamic pixel object
pub type DynamicPixelObjectCreationData = (
    PixelData,
    Vector2<f32>,
    Vector2<f32>,
    f32,
    f32,
    Vec<Collider>,
);

/// Dynamic pixel object components
pub type DynamicPixelObject = (
    RigidBodyHandle,
    PixelData,
    Vec<TempPixel>,
    Position,
    LinearVelocity,
    Angle,
    AngularVelocity,
);

/// Invisible object components
pub type InvisibleObject = (RigidBodyHandle, Position, Angle);

/// Utility function to update dynamic pixel object params based on rigid body
pub(crate) fn update_after_physics(
    rb: &RigidBody,
    pos: &mut Vector2<f32>,
    lin_vel: &mut Vector2<f32>,
    angle: &mut f32,
    ang_vel: &mut f32,
) {
    if rb.is_sleeping() {
        return;
    }
    let phys_pos = rb.position();
    let phys_angle = phys_pos.rotation.angle();
    let lv = rb.linvel();
    let av = rb.angvel();
    *pos = Vector2::new(phys_pos.translation.x, phys_pos.translation.y);
    *angle = phys_angle;
    *lin_vel = Vector2::new(lv.x, lv.y);
    *ang_vel = av;
}

pub(crate) fn dynamic_pixel_object(
    id: Entity,
    physics: &mut Physics,
    pixel_data: PixelData,
    pos: Vector2<f32>,
    lin_vel: Vector2<f32>,
    angle: f32,
    ang_vel: f32,
    generated_colliders: Vec<Collider>,
) -> DynamicPixelObject {
    let rb = DynamicRigidbody::new(
        id,
        &mut physics.bodies,
        &mut physics.colliders,
        pos,
        lin_vel,
        angle,
        ang_vel,
        generated_colliders,
    );
    (
        rb,
        pixel_data,
        vec![],
        Position(pos),
        LinearVelocity(lin_vel),
        Angle(angle),
        AngularVelocity(ang_vel),
    )
}

pub(crate) fn invisible_static_object(
    id: Entity,
    physics: &mut Physics,
    pos: Vector2<f32>,
    angle: f32,
    generated_colliders: Vec<Collider>,
) -> InvisibleObject {
    let rb = StaticRigidbody::new(
        id,
        &mut physics.bodies,
        &mut physics.colliders,
        pos,
        angle,
        generated_colliders,
    );
    (rb, Position(pos), Angle(angle))
}

pub(crate) fn invisible_sensor_object(
    id: Entity,
    physics: &mut Physics,
    pos: Vector2<f32>,
    angle: f32,
    generated_colliders: Vec<Collider>,
) -> InvisibleObject {
    let rb = SensorRigidbody::new(
        id,
        &mut physics.bodies,
        &mut physics.colliders,
        pos,
        angle,
        generated_colliders,
    );
    (rb, Position(pos), Angle(angle))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PixelObjectSaveDataArray {
    pub objects: Vec<PixelObjectSaveData>,
}

impl PixelObjectSaveDataArray {
    pub fn serialize(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn deserialize(data: &str) -> PixelObjectSaveDataArray {
        let deserialized: PixelObjectSaveDataArray = serde_json::from_str(data).unwrap();
        deserialized
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct PixelObjectSaveData {
    pub id: u32,
    pub pos: Vector2<f32>,
    pub angle: f32,
    pub lin_vel: Vector2<f32>,
    pub ang_vel: f32,
    pub matter: u32,
}

impl PixelObjectSaveData {
    /// Remember to add the object to world objects... (it gets only added to physics world...)
    pub fn add_dynamic_pixel_object(
        &self,
        ecs_world: &mut World,
        physics_world: &mut PhysicsWorld,
        simulation: &mut Simulation,
        image: &Arc<BitmapImage>,
    ) -> Result<Entity> {
        simulation.add_dynamic_pixel_object(
            ecs_world,
            physics_world,
            image,
            self.matter,
            self.pos,
            self.lin_vel,
            self.angle,
            self.ang_vel,
        )
    }

    pub fn from_dynamic_pixel_object(
        id: Entity,
        object_data: (PixelData, Position, LinearVelocity, Angle, AngularVelocity),
    ) -> PixelObjectSaveData {
        let (pixel_data, pos, lin_vel, angle, ang_vel) = object_data;
        let lin_vel = lin_vel.0;
        let ang_vel = ang_vel.0;
        let lin_vel = Vector2::new(lin_vel[0], lin_vel[1]);
        let matter = pixel_data
            .pixels
            .iter()
            .find(|p| p.matter != 0)
            .unwrap_or(&MatterPixel {
                matter: 0,
                color_index: 0,
                is_alive: false,
            })
            .matter;

        PixelObjectSaveData {
            id: id.id(),
            matter,
            pos: pos.0,
            angle: angle.0,
            lin_vel,
            ang_vel,
        }
    }

    #[allow(unused)]
    pub fn deserialize(data: &str) -> PixelObjectSaveData {
        let deserialized: PixelObjectSaveData = serde_json::from_str(data).unwrap();
        deserialized
    }

    #[allow(unused)]
    pub fn serialize(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
