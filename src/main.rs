use belly::prelude::*;
use bevy::{input::mouse::MouseMotion, prelude::*, transform::TransformSystem};
use bevy_dolly::prelude::*;
use bevy_hanabi::prelude::*;
use bevy_xpbd_3d::prelude::*;
use character_controller::*;
use game_management::GameLayer;
use iyes_perf_ui::prelude::*;
use space_editor::prelude::*;

mod character_controller;
mod game_management;
mod projectile;

// The component tag used to parent to a Dolly Rig
#[derive(Component)]
struct MainCamera;

fn main() {
    App::new()
        .insert_resource(Msaa::default())
        .add_plugins((
            DefaultPlugins,
            SpaceEditorPlugin,
            BellyPlugin,
            DollyCursorGrab,
            CharacterControllerPlugin,
            HanabiPlugin,
            //PhysicsPlugins::default(),
        ))// we want Bevy to measure these values for us:
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::EntityCountDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::SystemInformationDiagnosticsPlugin)
        .add_plugins(PerfUiPlugin)
        .add_systems(Startup, setup)
        .add_systems(Startup, simple_editor_setup)
        //.add_systems(Startup, effects_setup.before(setup))
        // Systems that create Egui widgets should be run during the `CoreSet::Update` set,
        // or after the `EguiSet::BeginFrame` system (which belongs to the `CoreSet::PreUpdate` set).
        //.add_systems(Update, ui_example_system)
        .add_systems(Update, Dolly::<MainCamera>::update_active)
        .add_systems(
            PostUpdate,
            update_camera
                .after(PhysicsSet::Sync)
                .before(TransformSystem::TransformPropagate),
        )
        .add_systems(Update, projectile::mouse_input)
        .add_systems(Update, projectile::update_projectiles)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
) {
    // create a simple Perf UI with default settings
    // and all entries provided by the crate:
    //commands.spawn(PerfUiCompleteBundle::default());

    // Player
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Capsule3d::new(0.4, 1.0)),
            material: materials.add(Color::rgb(0.4, 0.5, 0.9)),
            transform: Transform::from_xyz(0.0, 1.5, 0.0),
            ..default()
        },
        CharacterControllerBundle::new(Collider::capsule(1.0, 0.4)).with_movement(
            100.0,
            0.92,
            8.0,
            (70f32).to_radians(),
        ),
        CollisionLayers::new(GameLayer::Player, [GameLayer::Enemy, GameLayer::Ground, GameLayer::Default]),
        Grounded::default(),
        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
        GravityScale(2.0),
    ));

    // A cube to move around
    commands.spawn((
        RigidBody::Dynamic,
        Collider::cuboid(1.0, 1.0, 1.0),
        CollisionLayers::new(GameLayer::Default, LayerMask::ALL),
        PbrBundle {
            mesh: meshes.add(Cuboid::default()),
            material: materials.add(Color::rgb(0.8, 0.1, 0.1)),
            transform: Transform::from_xyz(3.0, 2.0, 3.0),
            ..default()
        },
    ));

    // Environment (see `async_colliders` example for creating colliders from scenes)
    commands.spawn((
        SceneBundle {
            scene: assets.load("models/Scene.glb#Scene0"),
            transform: Transform::from_rotation(Quat::from_rotation_y(-std::f32::consts::PI * 0.5)),
            ..default()
        },
        AsyncSceneCollider::new(Some(ComputedCollider::ConvexHull)),
        CollisionLayers::new(GameLayer::Ground, LayerMask::ALL),
        RigidBody::Static,
    ));

    // Sun
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            color: Color::rgb(1.0, 1.0, 0.98),
            illuminance: 8000.,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 0.0, 0.0)
            .looking_at(Vec3::new(0.2, -1.0, 0.2), Vec3::Y),
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
        },
        RayCaster::new(Vec3::ZERO, Direction3d::X),
    ));
}

fn effects_setup(mut commands: Commands, mut effects: ResMut<Assets<EffectAsset>>) {
    // Define a color gradient from red to transparent black
    let mut gradient = Gradient::new();
    gradient.add_key(0.0, Vec4::new(1., 0., 0., 1.));
    gradient.add_key(1.0, Vec4::splat(0.));

    // Create a new expression module
    let mut module = Module::default();

    // On spawn, randomly initialize the position of the particle
    // to be over the surface of a sphere of radius 2 units.
    let init_pos = SetPositionSphereModifier {
        center: module.lit(Vec3::ZERO),
        radius: module.lit(0.05),
        dimension: ShapeDimension::Surface,
    };

    // Also initialize a radial initial velocity to 6 units/sec
    // away from the (same) sphere center.
    let init_vel = SetVelocitySphereModifier {
        center: module.lit(Vec3::ZERO),
        speed: module.lit(6.),
    };

    // Initialize the total lifetime of the particle, that is
    // the time for which it's simulated and rendered. This modifier
    // is almost always required, otherwise the particles won't show.
    let lifetime = module.lit(10.); // literal value "10.0"
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Every frame, add a gravity-like acceleration downward
    let accel = module.lit(Vec3::new(0., -3., 0.));
    let update_accel = AccelModifier::new(accel);

    // Create the effect asset
    let effect = EffectAsset::new(
        // Maximum number of particles alive at a time
        vec![32768],
        // Spawn at a rate of 5 particles per second
        Spawner::rate(5.0.into()),
        // Move the expression module into the asset
        module,
    )
    .with_name("MyEffect")
    .init(init_pos)
    .init(init_vel)
    .init(init_lifetime)
    .update(update_accel)
    // Render the particles with a color gradient over their
    // lifetime. This maps the gradient key 0 to the particle spawn
    // time, and the gradient key 1 to the particle death (10s).
    .render(ColorOverLifetimeModifier { gradient });

    // Insert into the asset system
    let effect_handle = effects.add(effect);

    // Particles test
    commands.spawn(ParticleEffectBundle {
        effect: ParticleEffect::new(effect_handle),
        transform: Transform::from_translation(Vec3::Y),
        ..Default::default()
    });
}

fn update_camera(
    q0: Query<&Transform, With<CharacterController>>,
    mut q1: Query<&mut Rig>,
    mut motion_evr: EventReader<MouseMotion>,
    time: Res<Time>,
    grab_config: Res<DollyCursorGrabConfig>,
    mut query: Query<(&mut RayCaster, &Transform), With<MainCamera>>,
) {
    let player = q0.single().to_owned();
    let mut rig = q1.single_mut();
    let speed: f32 = 20.;

    rig.driver_mut::<bevy_dolly::prelude::Position>().position =
        player.translation + Vec3::new(0., 1., 0.);

    if grab_config.visible {
        return;
    }

    for ev in motion_evr.read() {
        rig.driver_mut::<YawPitch>().rotate_yaw_pitch(
            -ev.delta.x * time.delta_seconds() * speed,
            -ev.delta.y * time.delta_seconds() * speed,
        );
    }

    let (mut caster, cam) = query.single_mut();
    caster.origin = cam.translation;
    caster.direction = cam.forward();
}
