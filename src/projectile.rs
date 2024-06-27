use bevy::prelude::*;
use bevy_xpbd_3d::{
    components::{CollisionLayers, LinearVelocity, RigidBody},
    math::Vector3,
    prelude::{Collider, RayCaster, RayHits},
};

use crate::game_management::GameLayer;
use crate::{CharacterController, MainCamera};

/// Base projectile component marker
#[derive(Component)]
pub struct Projectile {
    pub direction: Vector3,
    pub speed: f32,
    pub lifetime: f32,
}

impl Default for Projectile {
    fn default() -> Self {
        Self {
            direction: Vector3::new(1., 1., 1.),
            speed: 10.,
            lifetime: 1.,
        }
    }
}

/// Sends [`MovementAction`] events based on keyboard input.
pub fn mouse_input(
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<&Transform, With<CharacterController>>,
    query_camera: Query<&Transform, With<MainCamera>>,
    query_ray: Query<(&RayCaster, &RayHits), With<MainCamera>>,
) {
    if !mouse_input.pressed(MouseButton::Left) {
        return;
    }
    let pos = query.single().translation;

    let cam_transform = query_camera.single();
    let speed = 10f32;

    let max_aim_distance = 1000f32;
    let mut hit_location = cam_transform.translation + cam_transform.forward() * max_aim_distance;

    if !query_ray.is_empty() {
        let (ray, hits) = query_ray.single();
        let hit = hits.iter_sorted().next();
        if !hit.is_none() {
            let impact_time = hit.unwrap().time_of_impact;
            hit_location = ray.origin + *ray.direction * impact_time;
        }
    }

    let dir = hit_location - pos;

    commands.spawn((
        Projectile::default(),
        RigidBody::Kinematic,
        LinearVelocity {
            0: bevy::prelude::Vec3::from(dir.normalize()) * speed,
        },
        Collider::cuboid(1.0, 1.0, 1.0),
        CollisionLayers::new(
            GameLayer::Projectile,
            [GameLayer::Enemy, GameLayer::Ground, GameLayer::Default],
        ),
        PbrBundle {
            mesh: meshes.add(Cuboid::default()),
            material: materials.add(Color::rgb(0.1, 0.8, 0.1)),
            transform: Transform::from_xyz(pos.x, pos.y + 0.1, pos.z),
            ..default()
        },
    ));
}

pub fn update_projectiles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Projectile)>,
    time: Res<Time>,
) {
    for (entity, mut projectile) in &mut query {
        projectile.lifetime -= time.delta_seconds();
        if projectile.lifetime <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}
