use bevy::{
    color::palettes::css::*,
    core_pipeline::{bloom::Bloom, tonemapping::Tonemapping},
    // dev_tools::picking_debug::{DebugPickingMode, DebugPickingPlugin},
    ecs::system::SystemId,
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use bevy_persistent::Persistent;
use bevy_rand::prelude::*;
use rand_core::RngCore;

use crate::Settings;

use super::*;

#[derive(Debug, Component)]
struct OnEggScene;

// #[derive(Debug, Component)]
// struct EntityInteraction{
//     mode: EntityInteractionMode,
// }

#[derive(Debug, Component, Deref, DerefMut)]
struct CameraSensitivity(Vec2);

impl Default for CameraSensitivity {
    fn default() -> Self {
        Self(
            // These factors are just arbitrary mouse sensitivity values.
            // It's often nicer to have a faster horizontal sensitivity than vertical.
            // We use a component for them so that we can make them user-configurable at runtime
            // for accessibility reasons.
            // It also allows you to inspect them in an editor if you `Reflect` the component.
            Vec2::new(0.003, 0.002),
        )
    }
}

#[derive(Debug, Component)]
struct WorldModelCamera;

#[derive(Resource)]
struct SpawnStarSystem(SystemId);

pub fn egg_plugin(app: &mut App) {
    let spawn_star_system = app.register_system(spawn_star);

    app.add_plugins((
        EntropyPlugin::<WyRand>::default(),
        MeshPickingPlugin,
        // DebugPickingPlugin,
    ))
    .add_systems(
        OnEnter(GameState::Egg),
        (spawn_player, spawn_world, spawn_stars, cursor_grab),
    )
    .add_systems(
        Update,
        (move_player, move_stars).run_if(in_state(GameState::Egg)),
    )
    // .insert_resource(DebugPickingMode::Normal)
    .insert_resource(SpawnStarSystem(spawn_star_system));
}

// #[derive(Debug, Component, Deref, DerefMut)]
// struct Velocity(Vec3);

fn spawn_player(mut commands: Commands, camera_old: Single<Entity, With<Camera2d>>) {
    commands.entity(camera_old.into_inner()).despawn();

    commands.spawn((
        OnEggScene,
        Player,
        InteractTarget(None),
        CameraSensitivity::default(),
        Transform::from_xyz(0.0, 1.0, 0.0),
        Visibility::default(),
        children![(
            WorldModelCamera,
            Camera3d::default(),
            Camera {
                hdr: true,
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            Tonemapping::TonyMcMapface,
            Bloom::NATURAL,
            Projection::from(PerspectiveProjection {
                fov: 70.0_f32.to_radians(),
                ..default()
            }),
        )],
    ));
}

const ROOM_BOUNDARY_MIN: Vec3 = Vec3::splat(-1.35);
const ROOM_BOUNDARY_MAX: Vec3 = Vec3::splat(1.35);

const PLAYER_STEP: f32 = 0.04;

fn move_player(
    settings: Res<Persistent<Settings>>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    key_input: Res<ButtonInput<KeyCode>>,
    player: Single<(&mut Transform, &CameraSensitivity), With<Player>>,
) {
    let (mut transform, camera_sensitivity) = player.into_inner();

    let mouse_delta = accumulated_mouse_motion.delta;
    let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);

    if mouse_delta != Vec2::ZERO {
        // Note that we are not multiplying by delta_time here.
        // The reason is that for mouse movement, we already get the full movement that happened since the last frame.
        // This means that if we multiply by delta_time, we will get a smaller rotation than intended by the user.
        // This situation is reversed when reading e.g. analog input from a gamepad however, where the same rules
        // as for keyboard input apply. Such an input should be multiplied by delta_time to get the intended rotation
        // independent of the framerate.
        let delta_yaw = -mouse_delta.x * camera_sensitivity.x;
        let delta_pitch = -mouse_delta.y * camera_sensitivity.y;

        let yaw = yaw + delta_yaw;

        // If the pitch was ±¹⁄₂ π, the camera would look straight up or down.
        // When the user wants to move the camera back to the horizon, which way should the camera face?
        // The camera has no way of knowing what direction was "forward" before landing in that extreme position,
        // so the direction picked will for all intents and purposes be arbitrary.
        // Another issue is that for mathematical reasons, the yaw will effectively be flipped when the pitch is at the extremes.
        // To not run into these issues, we clamp the pitch to a safe range.
        const PITCH_LIMIT: f32 = std::f32::consts::FRAC_PI_2 - 0.01;
        let pitch = (pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);

        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
    }

    // Forward movement from player's horizontal direction instead of forward() directly which allows for vertical movement
    // This probably looks terrible to someone who knows what they're doing
    // let Vec2 {x: yaw_x, y: yaw_y} = Vec2::from_angle(yaw);
    let forward = Vec3::new(-ops::sin(yaw), 0.0, -ops::cos(yaw)) * PLAYER_STEP;

    // Camera will never roll so sideways directions will never contain a vertical component
    let left = transform.left().as_vec3() * PLAYER_STEP * 0.5;

    let mut next_position = transform.translation;

    if key_input.pressed(settings.up) {
        next_position += forward;
    }
    if key_input.pressed(settings.down) {
        next_position -= forward;
    }
    if key_input.pressed(settings.left) {
        next_position += left;
    }
    if key_input.pressed(settings.right) {
        next_position -= left;
    }
    if key_input.pressed(settings.jump) {
        todo!()
    }

    transform.translation = next_position.clamp(ROOM_BOUNDARY_MIN, ROOM_BOUNDARY_MAX);
}

fn spawn_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let floor = meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(1.5)));
    let ceiling = meshes.add(Plane3d::new(Vec3::NEG_Y, Vec2::splat(1.5)));

    let wall_west = meshes.add(Plane3d::new(Vec3::X, Vec2::splat(1.5)));
    let wall_east = meshes.add(Plane3d::new(Vec3::NEG_X, Vec2::splat(1.5)));
    let wall_north = meshes.add(Plane3d::new(Vec3::Z, Vec2::splat(1.5)));
    // let wall_south_upper = meshes.add(Plane3d::new(Vec3::NEG_Z, Vec2::new(1.5, 0.5)));
    // let wall_south_lower = meshes.add(Plane3d::new(Vec3::NEG_Z, Vec2::new(1.5, 0.4)));
    // let wall_south_right = meshes.add(Plane3d::new(Vec3::NEG_Z, Vec2::new(0.4, 0.6)));
    // let wall_south_left = meshes.add(Plane3d::new(Vec3::NEG_Z, Vec2::new(0.1, 0.6)));

    // let window_inner_upper = meshes.add(Plane3d::new(Vec3::NEG_Y, Vec2::new(1.0, 0.025)));
    // let window_inner_right = meshes.add(Plane3d::new(Vec3::NEG_X, Vec2::new(0.6, 0.025)));
    // let window_inner_left = meshes.add(Plane3d::new(Vec3::X, Vec2::new(0.6, 0.025)));
    let wall_south_upper = meshes.add(Cuboid::new(3.0, 1.0, 0.05));
    let wall_south_lower = meshes.add(Cuboid::new(3.0, 0.8, 0.05));
    let wall_south_right = meshes.add(Cuboid::new(0.8, 1.2, 0.05));
    let wall_south_left = meshes.add(Cuboid::new(0.2, 1.2, 0.05));
    let window_sill = meshes.add(Cuboid::new(2.0, 0.01, 0.1));

    let bed = meshes.add(Cuboid::new(2.4, 0.3, 1.0));

    let material = materials.add(Color::WHITE);

    let room_elements = [
        (ceiling, (0.0, 3.0, 0.0)),
        (floor, (0.0, 0.0, 0.0)),
        (wall_west, (-1.5, 1.5, 0.0)),
        (wall_east, (1.5, 1.5, 0.0)),
        (wall_north, (0.0, 1.5, -1.5)),
        (wall_south_upper, (0.0, 2.5, 1.525)),
        (wall_south_lower, (0.0, 0.4, 1.525)),
        (wall_south_right, (-1.1, 1.4, 1.525)),
        (wall_south_left, (1.4, 1.4, 1.525)),
        // (wall_south_upper, (0.0, 2.5, 1.5)),
        // (wall_south_lower, (0.0, 0.4, 1.5)),
        // (wall_south_right, (-1.1, 1.4, 1.5)),
        // (wall_south_left, (1.4, 1.4, 1.5)),
        // (window_inner_upper, (0.3, 2.0, 1.525)),
        // (window_inner_right, (1.3, 1.4, 1.525)),
        // (window_inner_left, (-0.7, 1.4, 1.525)),
        (window_sill, (0.3, 0.8, 1.5)),
    ];

    room_elements.into_iter().for_each(|(mesh, (x, y, z))| {
        commands.spawn((
            OnEggScene,
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(x, y, z),
            Pickable::IGNORE,
        ));
    });

    // Window Glass
    commands.spawn((
        OnEggScene,
        Mesh3d(meshes.add(Cuboid::new(2.02, 1.202, 0.03))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            specular_transmission: 0.95,
            diffuse_transmission: 1.0,
            thickness: 0.03,
            ior: 1.49,
            perceptual_roughness: 0.0,
            reflectance: 0.5,
            ..default()
        })),
        Transform::from_xyz(0.3, 1.4, 1.525),
        EntityInteraction::Text(
            "Through eons of void, these photons birth from fusion, lay to rest in you.",
        ),
    ));

    commands.spawn((
        OnEggScene,
        Mesh3d(bed),
        MeshMaterial3d(material.clone()),
        Transform::from_xyz(-0.3, 0.16, -1.0),
        Pickable::IGNORE,
    ));

    // commands.spawn((
    //     Mesh3d(cube),
    //     MeshMaterial3d(material),
    //     Transform::from_xyz(0.75, 1.75, 0.0),
    // ));

    // Star light
    commands.spawn((
        OnEggScene,
        PointLight {
            color: Color::from(LAVENDER),
            intensity: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 1.4, 4.0),
        // The light source illuminates both the world model and the view model.
        // RenderLayers::from_layers(&[DEFAULT_RENDER_LAYER, VIEW_MODEL_RENDER_LAYER]),
    ));

    // Crack
    {
        let crack_material = materials.add(StandardMaterial {
            base_color_texture: Some(asset_server.load("bucko.png")),
            perceptual_roughness: 1.0,
            alpha_mode: AlphaMode::Mask(0.5),
            cull_mode: None,
            emissive: LinearRgba::rgb(150.0, 150.0, 150.0),
            ..default()
        });

        let mut crack_position = Transform::from_xyz(1.49, 1.0, 0.5);
        crack_position.rotate_local_y(-std::f32::consts::FRAC_PI_2);

        commands
            .spawn((
                OnEggScene,
                EntityInteraction::Special,
                Mesh3d(meshes.add(Rectangle::new(1.0, 1.0))),
                MeshMaterial3d(crack_material),
                crack_position,
            ))
            .observe(over_interactables)
            .observe(out_interactables);
    }

    // commands
    //     .spawn((
    //         Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 0.5))),
    //         MeshMaterial3d(materials.add(Color::WHITE)),
    //         Transform::from_xyz(-1.0, 1.0, 0.5),
    //     ))
    //     .observe(|_over: Trigger<Pointer<Over>>| {
    //         info!("Over cube");
    //     });
}

// Star with parallax speed
#[derive(Debug, Component)]
struct Star(f32);

const STAR_AMOUNT: usize = 200;
const MIN_STAR_HEIGHT: f32 = -15.0;
const MAX_STAR_HEIGHT: f32 = 30.0;
const MIN_STAR_LUMINANCE: f32 = 20.0;
const MAX_STAR_LUMINANCE: f32 = 200.0;
const MIN_STAR_SPEED: f32 = 0.01;
const MAX_STAR_SPEED: f32 = 0.5;

fn spawn_stars(mut commands: Commands, spawn_star: Res<SpawnStarSystem>) {
    (0..STAR_AMOUNT).for_each(|_| {
        commands.run_system(spawn_star.0);
    });
}

fn spawn_star(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: GlobalEntropy<WyRand>,
    mut count: Local<usize>,
) {
    let angle = random_range(rng.next_u32(), 0.0, std::f32::consts::PI);
    let mut height = MAX_STAR_HEIGHT;
    let lum = random_range(rng.next_u32(), MIN_STAR_LUMINANCE, MAX_STAR_LUMINANCE);
    let speed = random_range(rng.next_u32(), MIN_STAR_SPEED, MAX_STAR_SPEED);

    // Initial stars
    if *count != STAR_AMOUNT {
        height = random_range(rng.next_u32(), MIN_STAR_HEIGHT, MAX_STAR_HEIGHT);
        *count += 1;
    }

    let mut transform = Transform::from_xyz(10.0 * ops::cos(angle), height, 10.0 * ops::sin(angle));
    transform.rotate_local_y(3.0 * std::f32::consts::FRAC_PI_2 - angle);

    let material_emissive = materials.add(StandardMaterial {
        emissive: LinearRgba::rgb(lum, lum, lum),
        ..default()
    });

    commands.spawn((
        OnEggScene,
        Star(speed),
        Mesh3d(meshes.add(Circle::new(0.01))),
        MeshMaterial3d(material_emissive),
        Pickable::IGNORE,
        transform,
    ));
}

fn move_stars(
    mut commands: Commands,
    stars: Query<(Entity, &Star, &mut Transform)>,
    spawn_star: Res<SpawnStarSystem>,
) {
    for (entity, Star(speed), mut transform) in stars {
        transform.translation.y -= speed;
        if transform.translation.y <= MIN_STAR_HEIGHT {
            commands.entity(entity).despawn();
            commands.run_system(spawn_star.0);
        }
    }
}

fn cursor_grab(q_windows: Single<&mut Window, With<PrimaryWindow>>) {
    let mut primary_window = q_windows.into_inner();
    primary_window.cursor_options.grab_mode = CursorGrabMode::Locked;
    primary_window.cursor_options.visible = false;
}

fn _cursor_ungrab(q_windows: Single<&mut Window, With<PrimaryWindow>>) {
    let mut primary_window = q_windows.into_inner();
    primary_window.cursor_options.grab_mode = CursorGrabMode::None;
    primary_window.cursor_options.visible = true;
}

// Returns an f32 between two from a randomly generated u32
// From quad_rand <3
fn random_range(rand: u32, low: f32, high: f32) -> f32 {
    let r = rand as f64 / (u32::MAX as f64 + 1.0);
    let r = low as f64 + (high as f64 - low as f64) * r;
    r as f32
}
