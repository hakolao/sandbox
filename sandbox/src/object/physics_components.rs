use cgmath::Vector2;
use hecs::Entity;
use rapier2d::{parry::transformation::vhacd::VHACDParameters, prelude::*};

#[allow(unused)]
pub fn collider_from_mesh(vertices: &[Vector2<f32>], indices: &[[u32; 3]]) -> Collider {
    ColliderBuilder::trimesh(
        vertices.iter().map(|v| point![v.x, v.y]).collect(),
        indices.to_vec(),
    )
    .build()
}

#[allow(unused)]
pub fn collider_sensor_from_mesh(vertices: &[Vector2<f32>], indices: &[[u32; 3]]) -> Collider {
    ColliderBuilder::trimesh(
        vertices.iter().map(|v| point![v.x, v.y]).collect(),
        indices.to_vec(),
    )
    .sensor(true)
    .build()
}

pub fn collider_from_convex_decomposition(vertices: &[Vector2<f64>]) -> Collider {
    let verts = vertices
        .iter()
        .map(|v| point![v.x as f32, v.y as f32])
        .collect::<Vec<Point<Real>>>();
    let indices = (0..vertices.len() as u32 - 1)
        .map(|i| [i, i + 1])
        .collect::<Vec<[u32; 2]>>();
    ColliderBuilder::convex_decomposition_with_params(&verts, &indices, &VHACDParameters {
        resolution: 32,
        ..VHACDParameters::default()
    })
    .build()
}

pub fn collider_from_polylines(vertices: &[Vector2<f64>]) -> Collider {
    let verts = vertices
        .iter()
        .map(|v| point![v.x as f32, v.y as f32])
        .collect();
    ColliderBuilder::polyline(verts, None)
        .active_collision_types(ActiveCollisionTypes::default())
        .active_events(ActiveEvents::CONTACT_EVENTS | ActiveEvents::INTERSECTION_EVENTS)
        .build()
}

pub fn collider_sensor_from_polylines(vertices: &[Vector2<f64>]) -> Collider {
    let verts = vertices
        .iter()
        .map(|v| point![v.x as f32, v.y as f32])
        .collect();
    ColliderBuilder::polyline(verts, None)
        .sensor(true)
        .active_events(ActiveEvents::INTERSECTION_EVENTS)
        .build()
}

#[derive(Debug)]
pub struct DynamicRigidbody;

impl DynamicRigidbody {
    pub fn spawn(
        id: Entity,
        bodies: &mut RigidBodySet,
        collider_set: &mut ColliderSet,
        position: Vector2<f32>,
        lin_vel: Vector2<f32>,
        rotation: f32,
        ang_vel: f32,
        colliders: Vec<Collider>,
    ) -> RigidBodyHandle {
        let rigid_body = RigidBodyBuilder::new_dynamic()
            .translation(vector![position.x, position.y])
            .rotation(rotation)
            .linvel(vector![lin_vel.x, lin_vel.y])
            .angvel(ang_vel)
            .user_data(u64::from(id.to_bits()) as u128)
            .build();
        let rigid_body_handle = bodies.insert(rigid_body);
        for collider in colliders {
            collider_set.insert_with_parent(collider, rigid_body_handle, bodies);
        }
        rigid_body_handle
    }
}

#[derive(Debug)]
pub struct SensorRigidbody;

impl SensorRigidbody {
    pub fn spawn(
        id: Entity,
        bodies: &mut RigidBodySet,
        collider_set: &mut ColliderSet,
        position: Vector2<f32>,
        rotation: f32,
        colliders: Vec<Collider>,
    ) -> RigidBodyHandle {
        let rigid_body = RigidBodyBuilder::new_kinematic_position_based()
            .translation(vector![position.x, position.y])
            .rotation(rotation)
            .user_data(u64::from(id.to_bits()) as u128)
            .build();
        let rigid_body_handle = bodies.insert(rigid_body);
        for collider in colliders {
            collider_set.insert_with_parent(collider, rigid_body_handle, bodies);
        }
        rigid_body_handle
    }
}

#[derive(Debug)]
pub struct StaticRigidbody;

impl StaticRigidbody {
    pub fn spawn(
        id: Entity,
        bodies: &mut RigidBodySet,
        collider_set: &mut ColliderSet,
        position: Vector2<f32>,
        rotation: f32,
        colliders: Vec<Collider>,
    ) -> RigidBodyHandle {
        let rigid_body = RigidBodyBuilder::new_static()
            .translation(vector![position.x, position.y])
            .rotation(rotation)
            .user_data(u64::from(id.to_bits()) as u128)
            .build();
        let rigid_body_handle = bodies.insert(rigid_body);
        for collider in colliders {
            collider_set.insert_with_parent(collider, rigid_body_handle, bodies);
        }
        rigid_body_handle
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Position(pub Vector2<f32>);

#[derive(Debug, Copy, Clone)]
pub struct LinearVelocity(pub Vector2<f32>);

#[derive(Debug, Copy, Clone)]
pub struct AngularVelocity(pub f32);

#[derive(Debug, Copy, Clone)]
pub struct Angle(pub f32);
