use bevy::audio::Volume;
use bevy::prelude::*;

use crate::{
    RENDER_LAYER_OVERLAY, RENDER_LAYER_SPECIAL, RENDER_LAYER_WORLD, animation::*, audio::*,
    auto_scaling::AspectRatio, despawn_screen, game::interactions::*, progress::*,
};

use super::*;

use cracking::egg_cracking_plugin;
use stars::egg_stars_plugin;

mod cracking;
mod stars;

#[derive(Debug, Component)]
struct OnEggScene;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, SubStates)]
#[source(GameState = GameState::Egg)]
enum EggState {
    #[default]
    Loading,
    Ready,
    Cracking,
}

pub fn egg_plugin(app: &mut App) {
    use bevy::window::WindowResized;

    app.add_plugins((egg_cracking_plugin, egg_stars_plugin));

    app.add_sub_state::<EggState>();

    app.add_systems(
        OnEnter(GameState::Egg),
        (setup_pointer, setup_player, setup_world, cursor_grab),
    )
    .add_systems(
        OnExit(GameState::Egg),
        (
            despawn_screen::<OnEggScene>,
            audio_fade_out::<Ambience>,
            cursor_ungrab,
        ),
    )
    .add_systems(Update, update_pointer.run_if(on_event::<WindowResized>))
    .add_systems(Update, wait_till_loaded.run_if(in_state(EggState::Loading)))
    .add_systems(
        OnEnter(EggState::Ready),
        (
            initialize_crt_panel,
            initialize_interaction_observers,
            effects::fade_from_black,
            enable_movement,
        )
            .chain(),
    )
    .add_systems(
        Update,
        (
            move_player,
            get_egg_interactions
                .pipe(play_interactions)
                .run_if(just_pressed_interact),
        )
            .run_if(in_state(EggState::Ready).and(in_state(MovementEnabled))),
    );
}

fn wait_till_loaded(
    asset_server: Res<AssetServer>,
    mut asset_tracker: ResMut<AssetTracker>,
    mut egg_state: ResMut<NextState<EggState>>,
) {
    if asset_tracker.is_ready(asset_server) {
        egg_state.set(EggState::Ready);
    }
}

// #[derive(Debug, Component, Deref, DerefMut)]
// struct Velocity(Vec3);

#[derive(Debug, Component)]
struct EggPointer;

fn setup_pointer(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    primary_window: Single<(Entity, &Window), With<PrimaryWindow>>,
) {
    use bevy::asset::uuid::Uuid;
    use bevy::picking::pointer::*;
    use bevy::render::camera::NormalizedRenderTarget;
    use bevy::window::WindowRef;

    let (window_entity, window) = primary_window.into_inner();
    let center = window.size() / 2.0;

    commands.spawn((
        OnEggScene,
        Name::new("EggCrosshair"),
        Sprite::from_image(asset_server.load("pointer.png")),
        Transform::default().with_scale(Vec3::splat(0.8)),
        RENDER_LAYER_OVERLAY,
    ));

    commands.spawn((
        OnEggScene,
        EggPointer,
        Name::new("EggPointer"),
        PointerId::Custom(Uuid::new_v4()),
        PointerLocation::new(Location {
            target:   NormalizedRenderTarget::Window(
                WindowRef::Primary.normalize(Some(window_entity)).unwrap(),
            ),
            position: center,
        }),
    ));
}

use bevy::picking::pointer::PointerLocation;

fn update_pointer(
    mut pointer: Single<&mut PointerLocation, With<EggPointer>>,
    primary_window: Single<&Window, With<PrimaryWindow>>,
) {
    if let Some(location) = pointer.location.as_mut() {
        location.position = primary_window.size() / 2.0;
    }
}

fn setup_player(mut commands: Commands) {
    use bevy::core_pipeline::{bloom::Bloom, tonemapping::Tonemapping};

    info!("Spawning player");

    commands.spawn((
        OnEggScene,
        Player,
        Name::new("Player"),
        InteractTarget::default(),
        CameraSensitivity::default(),
        Transform::from_xyz(0.0, 1.0, 0.0),
        Visibility::default(),
        SpatialListener::default(),
        children![(
            WorldCamera,
            MeshPickingCamera,
            Camera3d::default(),
            Camera {
                order: 0,
                hdr: true,
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            Tonemapping::ReinhardLuminance,
            Bloom::OLD_SCHOOL,
            Projection::from(PerspectiveProjection {
                fov: 70.0_f32.to_radians(),
                ..default()
            }),
            AspectRatio(16.0 / 9.0),
            RENDER_LAYER_WORLD,
        )],
    ));
}

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut asset_tracker: ResMut<AssetTracker>,
    asset_server: Res<AssetServer>,
) {
    info!("Spawning egg world");

    let scene_room = asset_server.load(GltfAssetLabel::Scene(0).from_asset("egg.glb"));
    asset_tracker.push(scene_room.clone_weak().untyped());
    commands.spawn((OnEggScene, SceneRoot(scene_room)));

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
        PICKABLE,
        EntityInteraction::Text(
            "Through eons of void, these photons birth from fusion, lay to rest in you."
                .to_string(),
        ),
    ));

    // Star light
    commands.spawn((
        OnEggScene,
        DirectionalLight {
            color: Color::from(LAVENDER),
            illuminance: 4.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 1.4, 4.0),
    ));

    commands.spawn((
        OnEggScene,
        PointLight {
            intensity: 8192.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 2.9, 0.0),
    ));

    let hum = asset_server.load("audio/amb/hum.ogg");
    asset_tracker.push(hum.clone().untyped());

    commands.spawn((
        Ambience,
        Name::new("Egg Ambience"),
        AudioPlayer::new(hum),
        PlaybackSettings::LOOP.with_volume(Volume::SILENT),
        AudioFadeIn,
    ));
}

#[derive(Debug, Component)]
struct CrtPanel;

use bevy::gltf::GltfMaterialName;
fn initialize_crt_panel(
    q_gltf_materials: Query<(Entity, &GltfMaterialName, &MeshMaterial3d<StandardMaterial>)>,
    special_camera: Single<&Camera, With<SpecialCamera>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    if let Some((entity, material_handle)) = q_gltf_materials
        .iter()
        .find_map(|(entity, name, handle)| (name.0 == "CRT_Panel").then_some((entity, handle)))
    {
        if let Some(material) = materials.get_mut(&material_handle.0) {
            if let Some(render_image) = special_camera.target.as_image() {
                material.base_color = BLACK.into();
                material.emissive_texture = Some(render_image.clone());
                info!("CRT Panel Texture set to Special Camera render target");
            }
        }
        commands.entity(entity).insert((
            CrtPanel,
            PICKABLE,
            EntityInteraction::Text("amogus".to_string()),
        ));
    }
}

fn initialize_interaction_observers(
    q_interactables: Query<Entity, With<EntityInteraction>>,
    mut commands: Commands,
) {
    let mut observer_over = Observer::new(over_interactables);
    let mut observer_out = Observer::new(out_interactables);

    q_interactables.iter().for_each(|entity| {
        observer_over.watch_entity(entity);
        observer_out.watch_entity(entity);
    });

    commands.spawn(observer_over);
    commands.spawn(observer_out);
}

const ROOM_BOUNDARY: Vec3 = Vec3::splat(1.3);
const PLAYER_STEP: f32 = 0.04;

#[derive(Debug, Component, Deref, DerefMut)]
struct CameraSensitivity(Vec2);

impl Default for CameraSensitivity {
    fn default() -> Self {
        Self(vec2(0.003, 0.002))
    }
}

use bevy::input::mouse::AccumulatedMouseMotion;

fn move_player(
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    user_input: Res<UserInput>,
    player: Single<(&mut Transform, &CameraSensitivity), With<Player>>,
) {
    let (mut transform, camera_sensitivity) = player.into_inner();

    let mouse_delta = accumulated_mouse_motion.delta;
    let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);

    if mouse_delta != Vec2::ZERO {
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

    if user_input.moving() {
        const VECTOR_MAP: Vec2 = vec2(0.5 * PLAYER_STEP, -PLAYER_STEP);

        let translation = Vec2::from_angle(-yaw)
            .rotate(user_input.last_valid_direction.as_vec2() * VECTOR_MAP)
            .extend(0.0)
            .xzy();

        let next_position = transform.translation + translation;

        transform.translation = next_position.clamp(-ROOM_BOUNDARY, ROOM_BOUNDARY);
        // transform.translation = next_position;
    }
}

fn over_interactables(
    trigger: Trigger<Pointer<Over>>,
    q_interactables: Query<Entity, With<EntityInteraction>>,
    mut interaction_target: Single<&mut InteractTarget, With<Player>>,
) {
    if trigger.pointer_id.is_mouse() {
        // Disregard mouse inputs
        return;
    }

    // info!("Hovering");
    if let Ok(target_entity) = q_interactables.get(trigger.target()) {
        // info!("Over Target: {}", target_entity);
        interaction_target.set(target_entity);
    }
}

fn out_interactables(
    trigger: Trigger<Pointer<Out>>,
    q_interactables: Query<Entity, With<EntityInteraction>>,
    mut interaction_target: Single<&mut InteractTarget, With<Player>>,
) {
    if trigger.pointer_id.is_mouse() {
        // Disregard mouse inputs
        return;
    }

    // info!("Not Hovering");
    if let Ok(_target_entity) = q_interactables.get(trigger.target()) {
        // info!("Out Target: {}", target_entity);
        interaction_target.clear();
    }
}

use bevy::picking::backend::PointerHits;
use bevy::picking::pointer::PointerId;
fn get_egg_interactions(
    interact_target: Single<&InteractTarget, With<Player>>,
    mut pointer_hits: EventReader<PointerHits>,
    egg_pointer_id: Single<&PointerId, With<EggPointer>>,
    q_interactables: Query<&EntityInteraction>,
) -> Option<EntityInteraction> {
    const INTERACTION_RANGE: f32 = 1.0;

    let target_entity = interact_target.get()?;

    let current_frame_hit = pointer_hits
        .read()
        .filter(|hit| &hit.pointer == *egg_pointer_id && hit.order == 0.0)
        .last()?;

    let hitdata = current_frame_hit
        .picks
        .iter()
        .find_map(|(entity, hitdata)| (entity == target_entity).then_some(hitdata))?;

    let target_in_range = hitdata.depth < INTERACTION_RANGE;

    target_in_range.then(|| q_interactables.get(*target_entity).ok().cloned())?
}
