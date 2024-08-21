use avian3d::prelude::*;
use bevy::{
    input::mouse::MouseMotion,
    pbr::{ExtendedMaterial, MaterialExtension, OpaqueRendererMethod},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
    transform::TransformSystem,
};
use bevy_dolly::prelude::*;
use bevy_hanabi::prelude::*;
use character_controller::*;
use game_management::GameLayer;
use sickle_ui::{prelude::*, SickleUiPlugin};

mod character_controller;
mod game_management;
mod projectile;

// The component tag used to parent to a Dolly Rig
#[derive(Component, Reflect, Clone)]
#[reflect(Component, Default)]
struct MainCamera;

impl Default for MainCamera {
    fn default() -> Self {
        Self {}
    }
}

fn main() {
    App::new()
        .insert_resource(Msaa::default())
        .add_plugins(DefaultPlugins)
        .add_plugins(SickleUiPlugin)
        .add_plugins(DollyCursorGrab)
        .add_plugins(CharacterControllerPlugin)
        .add_plugins(HanabiPlugin)
        .add_plugins(PhysicsPlugins::default())
        //.add_plugins(EditorPlugin::default())
        //.add_plugins(EditorPlugin::new().in_new_window(Window::default()))
        .add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, MyExtension>,
        >::default())
        .add_systems(Startup, setup)
        //.add_systems(Startup, effects_setup.before(setup))
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
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, MyExtension>>>,
    assets: Res<AssetServer>,
) {
    // Player
    commands.spawn((
        MaterialMeshBundle {
            mesh: meshes.add(Capsule3d::new(0.5, 1.0)),
            transform: Transform::from_xyz(0.0, 1.0, 0.0),
            material: materials.add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color: Color::srgb(0.1, 0.1, 0.9),
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
        CharacterControllerBundle::new(Collider::capsule(1.0, 0.4)).with_movement(
            100.0,
            0.92,
            8.0,
            (70f32).to_radians(),
        ),
        CollisionLayers::new(
            GameLayer::Player,
            [GameLayer::Enemy, GameLayer::Ground, GameLayer::Default],
        ),
        Grounded::default(),
        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
        GravityScale(2.0),
    ));

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
        RayCaster::new(Vec3::ZERO, Dir3::X),
    ));

    // Ground
    commands.spawn((
        MaterialMeshBundle {
            mesh: meshes.add(Plane3d::new(Vec3::Y, Vec2::new(10.0, 10.0))),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            material: materials.add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color: Color::srgb(0.2, 0.9, 0.2),
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
        CollisionLayers::new(GameLayer::Ground, LayerMask::ALL),
        RigidBody::Static,
        Collider::cuboid(20.0, 0.0, 20.0),
    ));

    // light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::srgb(0.8, 0.8, 0.1),
            shadows_enabled: true,
            illuminance: 30000.0,
            ..default()
        },
        transform: Transform::default().looking_at(Vec3::new(-1.0, -2.5, -1.5), Vec3::Y),
        ..default()
    });
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

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct MyExtension {
    // We need to ensure that the bindings of the base material and the extension do not conflict,
    // so we start from binding slot 100, leaving slots 0-99 for the base material.
    #[uniform(100)]
    pub quantize_steps: u32,
}

impl MaterialExtension for MyExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/toon_shader.wgsl".into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/toon_shader.wgsl".into()
    }
}
