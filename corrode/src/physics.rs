use cgmath::Vector2;
use rapier2d::prelude::*;
use rayon::ThreadPool;

pub struct Physics {
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
    pub joints: JointSet,
    pub physics_pipeline: PhysicsPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: BroadPhase,
    pub narrow_phase: NarrowPhase,
    pub ccd_solver: CCDSolver,
    pub gravity: Vector<f32>,
    pub query_pipeline: QueryPipeline,
    pub integration_parameters: IntegrationParameters,
}

impl Default for Physics {
    fn default() -> Self {
        Physics::new()
    }
}

impl Physics {
    pub fn new() -> Physics {
        let bodies = RigidBodySet::new();
        let colliders = ColliderSet::new();
        let joints = JointSet::new();
        let integration_parameters = IntegrationParameters::default();
        let physics_pipeline = PhysicsPipeline::new();
        let island_manager = IslandManager::new();
        let broad_phase = BroadPhase::new();
        let narrow_phase = NarrowPhase::new();
        let ccd_solver = CCDSolver::new();

        Physics {
            bodies,
            colliders,
            joints,
            physics_pipeline,
            island_manager,
            broad_phase,
            narrow_phase,
            ccd_solver,
            gravity: Vector::y() * -9.81,
            query_pipeline: QueryPipeline::new(),
            integration_parameters,
        }
    }
}

pub struct PhysicsWorld {
    pub physics: Physics,
    event_handler: ChannelEventCollector,
    contact_recv: crossbeam::channel::Receiver<ContactEvent>,
    intersection_recv: crossbeam::channel::Receiver<IntersectionEvent>,
}

impl PhysicsWorld {
    pub fn new() -> PhysicsWorld {
        let (contact_send, contact_recv) = crossbeam::channel::unbounded();
        let (intersection_send, intersection_recv) = crossbeam::channel::unbounded();
        let event_handler = ChannelEventCollector::new(intersection_send, contact_send);
        PhysicsWorld {
            physics: Physics::new(),
            event_handler,
            contact_recv,
            intersection_recv,
        }
    }

    pub fn step(
        &mut self,
        thread_pool: &ThreadPool,
        intersection_event_handler: impl Fn(IntersectionEvent),
        contact_event_handler: impl Fn(ContactEvent),
    ) {
        let Physics {
            gravity,
            integration_parameters,
            island_manager,
            broad_phase,
            narrow_phase,
            bodies,
            colliders,
            joints,
            ccd_solver,
            physics_pipeline,
            query_pipeline,
            ..
        } = &mut self.physics;
        let event_handler = &self.event_handler;
        thread_pool.install(|| {
            physics_pipeline.step(
                gravity,
                integration_parameters,
                island_manager,
                broad_phase,
                narrow_phase,
                bodies,
                colliders,
                joints,
                ccd_solver,
                &(),
                event_handler,
            );
        });

        query_pipeline.update(island_manager, bodies, colliders);

        while let Ok(intersection_event) = self.intersection_recv.try_recv() {
            intersection_event_handler(intersection_event);
        }
        while let Ok(contact_event) = self.contact_recv.try_recv() {
            contact_event_handler(contact_event);
        }
    }

    pub fn remove_physics(&mut self, rb: RigidBodyHandle) {
        let Physics {
            bodies,
            island_manager,
            colliders,
            joints,
            ..
        } = &mut self.physics;
        bodies.remove(rb, island_manager, colliders, joints);
    }

    pub fn rigid_body_at_pos(&self, world_pos: Vector2<f32>) -> Option<&RigidBody> {
        let Physics {
            colliders,
            query_pipeline,
            ..
        } = &self.physics;
        let ray = Ray::new(point![world_pos.x, world_pos.y], vector![0.0, 1.0]);
        let max_toi = 0.0;
        let solid = true;
        let groups = InteractionGroups::all();
        let filter = None;

        if let Some((handle, _toi)) =
            query_pipeline.cast_ray(colliders, &ray, max_toi, solid, groups, filter)
        {
            let collider = &colliders[handle];
            let rb_handle = collider.parent().unwrap();
            Some(&self.physics.bodies[rb_handle])
        } else {
            None
        }
    }

    #[allow(unused)]
    pub fn rigid_body_mut_at_pos(&mut self, world_pos: Vector2<f32>) -> Option<&mut RigidBody> {
        let Physics {
            colliders,
            query_pipeline,
            ..
        } = &mut self.physics;
        let ray = Ray::new(point![world_pos.x, world_pos.y], vector![0.0, 1.0]);
        let max_toi = 0.0;
        let solid = true;
        let groups = InteractionGroups::all();
        let filter = None;

        if let Some((handle, _toi)) =
            query_pipeline.cast_ray(colliders, &ray, max_toi, solid, groups, filter)
        {
            let collider = &colliders[handle];
            let rb_handle = collider.parent().unwrap();
            Some(&mut self.physics.bodies[rb_handle])
        } else {
            None
        }
    }
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        PhysicsWorld::new()
    }
}
