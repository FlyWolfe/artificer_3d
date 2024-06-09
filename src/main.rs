use bevy::{input::mouse::MouseMotion, prelude::*, transform::TransformSystem};
use bevy_dolly::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_xpbd_3d::prelude::*;
use character_controller::*;

mod character_controller;

// The component tag used to parent to a Dolly Rig
#[derive(Component)]
struct MainCamera;

fn main() {
    App::new()
        .insert_resource(Msaa::default())
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            EguiPlugin,
            CharacterControllerPlugin,
        ))
        .add_systems(Startup, setup)
        // Systems that create Egui widgets should be run during the `CoreSet::Update` set,
        // or after the `EguiSet::BeginFrame` system (which belongs to the `CoreSet::PreUpdate` set).
        .add_systems(Update, ui_example_system)
        .add_systems(
            PostUpdate,
            Dolly::<MainCamera>::update_active
                .after(PhysicsSet::Sync)
                .before(TransformSystem::TransformPropagate),
        )
        .add_systems(
            PostUpdate,
            update_camera
                .after(PhysicsSet::Sync)
                .before(TransformSystem::TransformPropagate),
        )
        .run();
}

fn ui_example_system(mut contexts: EguiContexts) {
    if true {
        return;
    }
    egui::Window::new("Hello").show(contexts.ctx_mut(), |ui| {
        ui.label("world");
    });
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
) {
    // Player
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Capsule3d::new(0.4, 1.0)),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6)),
            transform: Transform::from_xyz(0.0, 1.5, 0.0),
            ..default()
        },
        CharacterControllerBundle::new(Collider::capsule(1.0, 0.4)).with_movement(
            100.0,
            0.92,
            8.0,
            (30f32).to_radians(),
        ),
        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
        GravityScale(2.0),
    ));

    // A cube to move around
    commands.spawn((
        RigidBody::Dynamic,
        Collider::cuboid(1.0, 1.0, 1.0),
        PbrBundle {
            mesh: meshes.add(Cuboid::default()),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6)),
            transform: Transform::from_xyz(3.0, 2.0, 3.0),
            ..default()
        },
    ));

    // Environment (see `async_colliders` example for creating colliders from scenes)
    commands.spawn((
        SceneBundle {
            scene: assets.load("character_controller_demo.glb#Scene0"),
            transform: Transform::from_rotation(Quat::from_rotation_y(-std::f32::consts::PI * 0.5)),
            ..default()
        },
        AsyncSceneCollider::new(Some(ComputedCollider::ConvexHull)),
        RigidBody::Static,
    ));

    // Sun
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            color: Color::rgb(1.0, 1.0, 0.9),
            illuminance: 12000.,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::new(0.2, -1.0, 0.2), Vec3::Y),
        ..default()
    });

    // Camera
    commands.spawn((
        MainCamera,
        Rig::builder()
            .with(bevy_dolly::prelude::Position::new(Vec3::ZERO))
            .with(YawPitch::new().yaw_degrees(0.0).pitch_degrees(-30.0))
            .with(Smooth::new_position(0.3))
            .with(Smooth::new_rotation(0.3))
            .with(Arm::new((Vec3::Z * 10.0) + (Vec3::Y * 1.0)))
            .build(),
        Camera3dBundle {
            transform: Transform::from_xyz(0., 1., 5.).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        }
    ));
}

fn update_camera(q0: Query<&Transform, With<CharacterController>>, mut q1: Query<&mut Rig>, mut motion_evr: EventReader<MouseMotion>, time: Res<Time>) {
    let player = q0.single().to_owned();
    let mut rig = q1.single_mut();
    let speed: f32 = 20.;

    rig.driver_mut::<bevy_dolly::prelude::Position>()
        .position = player.translation + Vec3::new(0., 1., 0.);

    for ev in motion_evr.read() {
        rig.driver_mut::<YawPitch>()
        .rotate_yaw_pitch(-ev.delta.x * time.delta_seconds() * speed, -ev.delta.y * time.delta_seconds() * speed);
    }
    
}
