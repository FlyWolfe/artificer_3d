use avian3d::{
    collision::CollisionLayers,
    dynamics::rigid_body::{LinearVelocity, RigidBody},
    math::Vector3,
    prelude::{Collider, RayCaster, RayHits},
};
use bevy::{
    ecs::query::QuerySingleError,
    pbr::{ExtendedMaterial, OpaqueRendererMethod},
    prelude::*,
};

use crate::{game_management::GameLayer, MyExtension};
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
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, MyExtension>>>,
    query: Query<&Transform, With<CharacterController>>,
    query_camera: Query<&Transform, With<MainCamera>>,
    query_ray: Query<(&RayCaster, &RayHits), With<MainCamera>>,
) {
    if !mouse_input.pressed(MouseButton::Left) {
        return;
    }
    let mut pos = Vec3::ZERO;
    match query.get_single() {
        Ok(transform) => {
            pos = transform.translation;
        }
        Err(QuerySingleError::NoEntities(_)) => {
            println!("Error: There is no player!");
        }
        Err(QuerySingleError::MultipleEntities(_)) => {
            println!("Error: There is more than one player!");
        }
    }

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
        MaterialMeshBundle {
            mesh: meshes.add(Sphere::default()),
            transform: Transform::from_xyz(pos.x, pos.y + 0.1, pos.z),
            material: materials.add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color: Color::srgb(0.9, 0.1, 0.1),
                    // can be used in forward or deferred mode.
                    opaque_render_method: OpaqueRendererMethod::Auto,
                    // in deferred mode, only the PbrInput can be modified (uvs, color and other material properties),
                    // in forward mode, the output can also be modified after lighting is applied.
                    // see the fragment shader `extended_material.wgsl` for more info.
                    // Note: to run in deferred mode, you must also add a `DeferredPrepass` component to the camera and either
                    // change the above to `OpaqueRendererMethod::Deferred` or add the `DefaultOpaqueRendererMethod` resource.
                    ..Default::default()
                },
                extension: MyExtension { quantize_steps: 3 },
            }),
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
