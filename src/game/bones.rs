#![allow(clippy::type_complexity)]
use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::*;

use crate::{
    RENDER_LAYER_WORLD, WINDOW_HEIGHT, WINDOW_WIDTH, animation::SpriteAnimation, despawn_screen,
    game::effects::*,
};

use super::*;

#[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[source(GameState = GameState::Bones)]
enum BonesState {
    #[default]
    Loading,
    Playing,
    Ending,
}

#[derive(Debug, Component)]
struct OnBones;

pub fn bones_plugin(app: &mut App) {
    app.add_systems(
        OnEnter(BonesState::Loading),
        (
            fade_from_black,
            setup_camera,
            setup_map,
            setup_player,
            spawn_enemies,
        ),
    )
    .add_systems(
        Update,
        wait_till_loaded.run_if(in_state(BonesState::Loading)),
    )
    .add_systems(
        Update,
        (
            progress_map,
            update_buckos_grounded,
            (
                player_jump.run_if(just_pressed_jump.and(player_grounded)),
                update_player_animation,
            )
                .chain()
                .run_if(in_state(MovementState::Enabled)),
            update_enemies_velocity,
            enemy_sprint,
            enemy_chase,
            enemy_jump,
            enemy_dive,
            enemy_land_jump,
            enemy_land_dive,
            enemy_recover,
        )
            .run_if(in_state(BonesState::Playing)),
    )
    .add_systems(FixedUpdate, update_player_velocity.run_if(player_grounded))
    .add_systems(OnEnter(BonesState::Ending), fade_to_black)
    .add_systems(
        Update,
        conclude_bones.run_if(in_state(BonesState::Ending).and(on_event::<FadeIn>)),
    )
    .add_systems(
        OnExit(GameState::Bones),
        (despawn_screen::<OnBones>, fade_from_black),
    )
    // .add_systems(
    //     Update,
    //     (|state: Res<State<BonesState>>| info!("{:?}", **state))
    //         .run_if(state_changed::<BonesState>),
    // )
    .add_sub_state::<BonesState>()
    .init_resource::<BonesAssetTracker>()
    .init_resource::<BonesTimer>()
    .register_type::<BonesHealth>()
    .register_type::<Grounded>()
    .register_type::<UckoState>();
}

#[derive(Debug, Default, Deref, DerefMut, Resource)]
struct BonesAssetTracker(Vec<UntypedHandle>);

impl BonesAssetTracker {
    fn loaded(&self, asset_server: Res<'_, AssetServer>) -> bool {
        self.iter().all(|handle| {
            matches!(
                asset_server.get_load_state(handle.id()),
                Some(bevy::asset::LoadState::Loaded)
            )
        })
    }
}

fn setup_camera(mut commands: Commands) {
    use crate::auto_scaling::AspectRatio;
    use bevy::render::camera::ScalingMode;

    commands.spawn((
        OnBones,
        WorldCamera,
        Camera2d,
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Transform::from_xyz(WINDOW_WIDTH / 4.0, WINDOW_HEIGHT / 4.0, 0.0)
            .with_scale(Vec3::splat(0.5)),
        AspectRatio(16.0 / 9.0),
        Projection::from({
            OrthographicProjection {
                near: -1000.0,
                scaling_mode: ScalingMode::Fixed {
                    width:  WINDOW_WIDTH,
                    height: WINDOW_HEIGHT,
                },
                ..OrthographicProjection::default_3d()
            }
        }),
        RENDER_LAYER_WORLD,
    ));
}

#[derive(Debug, Default, PhysicsLayer)]
enum ColliderLayer {
    #[default]
    Default,
    Player,
    Enemy,
    World,
}

fn setup_map(
    mut commands: Commands,
    mut asset_tracker: ResMut<BonesAssetTracker>,
    asset_server: Res<AssetServer>,
) {
    let bones_map_handle = asset_server.load("maps/bones.tmx");
    asset_tracker.push(bones_map_handle.clone().untyped());

    let world_collision_layers = CollisionLayers::new(
        [ColliderLayer::World],
        [
            ColliderLayer::Default,
            ColliderLayer::Player,
            ColliderLayer::Enemy,
        ],
    );

    commands
        .spawn((TiledMapHandle(bones_map_handle), TilemapAnchor::BottomLeft))
        .insert(OnBones)
        .observe(
            move |trigger: Trigger<TiledColliderCreated>, mut commands: Commands| {
                commands.entity(trigger.entity).insert((
                    RigidBody::Static,
                    world_collision_layers,
                    Friction::ZERO,
                ));
            },
        );

    commands.insert_resource(Gravity(vec2(0.0, -9.81 * 32.0)));
}

#[derive(Debug, Component, Deref, DerefMut, Reflect)]
#[reflect(Component)]
struct BonesHealth(u8);

const PLAYER_MAX_HEALTH: u8 = 8;

#[derive(Debug, Component, Deref, DerefMut, PartialEq, Reflect)]
#[reflect(Component)]
struct Grounded(bool);

fn setup_player(
    mut commands: Commands,
    mut asset_tracker: ResMut<BonesAssetTracker>,
    asset_server: Res<AssetServer>,
) {
    const PLAYER_START: Vec3 = vec3(384.0, 175.0, 1.0);

    let player_image = asset_server.load("sprites/bucko/escape.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(64), 8, 8, None, None);
    let layout = asset_server.add(layout);
    asset_tracker.push(player_image.clone().untyped());

    let player_collision_layers = CollisionLayers::new(
        [ColliderLayer::Player],
        [
            ColliderLayer::Default,
            ColliderLayer::Enemy,
            ColliderLayer::World,
        ],
    );

    commands
        .spawn((
            OnBones,
            Player,
            Name::new("Bucko"),
            BonesHealth(PLAYER_MAX_HEALTH),
            Transform::from_translation(PLAYER_START),
            (
                // Visual
                Sprite::from_atlas_image(player_image, TextureAtlas { layout, index: 0 }),
                SpriteAnimation::new(0, 7, 12).looping(),
                Visibility::default(),
                RENDER_LAYER_WORLD,
            ),
            (
                // Physics
                RigidBody::Dynamic,
                Collider::capsule_endpoints(16.0, vec2(0.0, 8.0), vec2(0.0, -16.0)),
                CollisionEventsEnabled,
                player_collision_layers,
                LockedAxes::ROTATION_LOCKED,
                LinearVelocity(Vec2::X * 32.0),
                Friction::ZERO,
                LinearDamping(0.2),
                Grounded(true),
            ),
        ))
        .observe(player_damage);
}

#[derive(Debug, Component)]
struct Ucko;

#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component)]
enum UckoState {
    #[default]
    Chase,
    Sprint,
    Jump,
    Dive,
    Recover,
}

#[derive(Debug, Component, Deref, DerefMut, Reflect)]
#[reflect(Component)]
struct RecoveryTimer(Timer);

impl Default for RecoveryTimer {
    fn default() -> Self {
        const RECOVERY_TIME: f32 = 8.0;
        RecoveryTimer(Timer::from_seconds(RECOVERY_TIME, TimerMode::Once))
    }
}

#[derive(Debug, Component)]
struct UckoHitbox;

const ENEMY_AMOUNT: u8 = 3;

fn spawn_enemies(
    mut commands: Commands,
    mut asset_tracker: ResMut<BonesAssetTracker>,
    asset_server: Res<AssetServer>,
) {
    const ENEMY_START: Vec3 = vec3(64.0, 176.0, 1.0);
    const ENEMY_SPACING: f32 = 48.0;

    let enemy_image = asset_server.load("bucko.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(128), 1, 1, None, None);
    let layout = asset_server.add(layout);
    asset_tracker.push(enemy_image.clone().untyped());

    let enemy_sprite = Sprite {
        image: enemy_image,
        custom_size: Some(Vec2::splat(64.0)),
        texture_atlas: Some(layout.into()),
        ..default()
    };

    let enemy_collision_layers = CollisionLayers::new(
        [ColliderLayer::Enemy],
        [ColliderLayer::Default, ColliderLayer::World],
    );

    let hitbox_collision_layers = CollisionLayers::new(
        [ColliderLayer::Enemy],
        [ColliderLayer::Default, ColliderLayer::Player],
    );

    let raycaster_filter =
        SpatialQueryFilter::from_mask([ColliderLayer::World, ColliderLayer::Player]);

    (0..ENEMY_AMOUNT).for_each(move |i| {
        let offset = Vec3::NEG_X * ENEMY_SPACING * i as f32;
        commands.spawn((
            OnBones,
            Ucko,
            Name::new(format!("Ucko_{}", i)),
            UckoState::default(),
            RecoveryTimer::default(),
            Transform::from_translation(ENEMY_START + offset),
            (
                // Visual
                enemy_sprite.clone(),
                SpriteAnimation::set_frame(0),
                Visibility::default(),
                RENDER_LAYER_WORLD,
            ),
            (
                // Physics
                RigidBody::Dynamic,
                Collider::circle(16.0),
                RayCaster::new(Vec2::ZERO, Dir2::X)
                    .with_max_distance(64.0)
                    .with_query_filter(raycaster_filter.clone()),
                enemy_collision_layers,
                LockedAxes::ROTATION_LOCKED,
                LinearVelocity(Vec2::X * 36.0),
                Friction::ZERO,
                Grounded(true),
                children![(
                    UckoHitbox,
                    Sensor,
                    Collider::circle(15.9),
                    hitbox_collision_layers,
                )],
            ),
        ));
    });
}

fn wait_till_loaded(
    mut bones_state: ResMut<NextState<BonesState>>,
    mut asset_tracker: ResMut<BonesAssetTracker>,
    asset_server: Res<AssetServer>,
) {
    if asset_tracker.loaded(asset_server) {
        asset_tracker.clear();
        bones_state.set(BonesState::Playing);
    }
}

#[derive(Debug, Deref, DerefMut, Resource, Reflect)]
#[reflect(Resource)]
struct BonesTimer(Timer);

impl Default for BonesTimer {
    fn default() -> Self {
        BonesTimer(Timer::from_seconds(120.0, TimerMode::Once))
    }
}

const MAP_WIDTH: f32 = 4096.0;
const CAMERA_SPEED: f32 = 32.0; // pixels per second

fn progress_map(
    mut camera: Single<&mut Transform, With<WorldCamera>>,
    mut bones_state: ResMut<NextState<BonesState>>,
    mut timer: ResMut<BonesTimer>,
    time: Res<Time>,
) {
    const CAMERA_X_START: f32 = WINDOW_WIDTH / 4.0; // half window width * camera scale
    const CAMERA_X_END: f32 = MAP_WIDTH - CAMERA_X_START;

    let time_seconds = timer.tick(time.delta()).elapsed_secs();
    camera.translation.x = ((CAMERA_SPEED * time_seconds) + CAMERA_X_START).min(CAMERA_X_END);

    if timer.just_finished() {
        bones_state.set(BonesState::Ending);
    }
}

// fn set_player_max_speed(player: Single<(&mut BonesHealth, &mut LinearVelocity), With<Player>>) {}

fn player_damage(
    trigger: Trigger<OnCollisionStart>,
    q_hitboxes: Query<(), With<UckoHitbox>>,
    mut player_health: Single<&mut BonesHealth, With<Player>>,
) {
    if q_hitboxes.contains(trigger.collider) {
        player_health.0 = player_health.saturating_sub(1);
    }
}

fn update_buckos_grounded(
    collisions: Collisions,
    ground_bodies: Query<&RigidBody, With<TiledColliderMarker>>,
    mut buckos: Query<(Entity, &mut Grounded)>,
) {
    buckos.iter_mut().for_each(|(entity, mut grounded)| {
        let is_grounded = collisions.collisions_with(entity).any(|contact| {
            let is_first = entity == contact.collider1;

            let other_body = match is_first {
                true => contact.body2,
                false => contact.body1,
            };

            let Some(other_body) = other_body else {
                return false; // other collider has no body so it's not the ground
            };

            if !ground_bodies.contains(other_body) {
                return false;
            }

            contact.manifolds.iter().any(|manifold| {
                let normal = match is_first {
                    true => -manifold.normal,
                    false => manifold.normal,
                }; // Normal points from ground to bucko

                normal.y > 0.0
            }) // contact isn't completely horizontal, you're grounded bucko
        });

        grounded.set_if_neq(Grounded(is_grounded));
    })
}

fn player_grounded(player_grounded: Single<&Grounded, With<Player>>) -> bool {
    player_grounded.0
}

fn update_player_velocity(
    player: Single<(&BonesHealth, &mut LinearVelocity), With<Player>>,
    user_input: Res<UserInput>,
    time: Res<Time<Fixed>>,
) {
    const ACCELERATION: f32 = 256.0;

    let (health, mut velocity) = player.into_inner();

    let max_x_velocity = (health.0 as f32 * 2.0) + 32.0;

    // let health_multiplier = (health.0 as f32 / 8.0) + 1.0;
    // velocity.x = ((user_input.raw_vector.x * 16.0) + MIN_SPEED) * health_multiplier;
    velocity.x += user_input.raw_vector.x * ACCELERATION * time.delta_secs();
    velocity.x = velocity.x.clamp(0.0, max_x_velocity);
}

fn player_jump(mut player_velocity: Single<&mut LinearVelocity, With<Player>>) {
    const JUMP_IMPULSE: Vec2 = vec2(32.0, 256.0);
    // With this jank "grounded" check, you can probably
    // double jump if you're laggy enough
    // let dy_near_zero = velocity.y.abs() < 0.001;
    // if grounded.0 && matches!(user_input.jump, KeyState::Press) {
    //     velocity.y += JUMP_IMPULSE;
    // }
    player_velocity.0 += JUMP_IMPULSE;
}

fn update_player_animation(
    player: Single<
        (
            Ref<BonesHealth>,
            Ref<Grounded>,
            &LinearVelocity,
            &mut SpriteAnimation,
        ),
        With<Player>,
    >,
) {
    const ANIMATION_ROWS: u8 = 6;
    const SPRITES_PER_ROW: usize = 8;
    // const STILLS_INDEX: usize = 56;

    let (health, grounded, velocity, mut animator) = player.into_inner();

    let current_row = (PLAYER_MAX_HEALTH - health.0).min(ANIMATION_ROWS - 1) as usize;
    let new_index = SPRITES_PER_ROW * current_row;

    if health.is_changed() || grounded.is_changed() {
        *animator = match grounded.0 {
            true => SpriteAnimation::new(new_index, new_index + 7, 12).looping(),
            false => SpriteAnimation::set_frame(new_index),
        }
    }

    // match velocity.x < 4.0 {
    //     true => *animator = SpriteAnimation::set_frame(STILLS_INDEX + current_row),
    //     false => animator.as_mut().change_fps(6 + (velocity.x / 8.0) as u8),
    // }
    animator.as_mut().change_fps(6 + (velocity.x / 8.0) as u8);
}

fn update_enemies_velocity(
    mut q_enemies: Query<(&UckoState, &mut LinearVelocity), With<Ucko>>,
    time_fixed: Res<Time<Fixed>>,
) {
    const MAX_X_VELOCITY_NORMAL: f32 = 33.0;
    const MAX_X_VELOCITY_SPRINT: f32 = 48.0;
    const ACCELERATION: f32 = 256.0;

    q_enemies
        .iter_mut()
        .filter(|(state, ..)| matches!(**state, UckoState::Chase | UckoState::Sprint))
        .for_each(|(state, mut velocity)| {
            velocity.x += ACCELERATION * time_fixed.delta_secs();
            velocity.x = match state {
                UckoState::Chase => velocity.x.clamp(0.0, MAX_X_VELOCITY_NORMAL),
                UckoState::Sprint => velocity.x.clamp(0.0, MAX_X_VELOCITY_SPRINT),
                _ => unreachable!(),
            }
        });
}

fn enemy_sprint(
    camera: Single<&Transform, With<WorldCamera>>,
    mut q_enemies: Query<(&mut UckoState, &Transform), With<Ucko>>,
) {
    const OFFSCREEN: f32 = WINDOW_WIDTH / 4.0;
    let camera_position = camera.translation.x;

    q_enemies
        .iter_mut()
        .filter(|(state, ..)| matches!(**state, UckoState::Chase))
        .for_each(|(mut state, transform)| {
            let distance_to_center = camera_position - transform.translation.x;
            if distance_to_center > OFFSCREEN {
                *state = UckoState::Sprint;
            }
        });
}

fn enemy_chase(
    camera: Single<&Transform, With<WorldCamera>>,
    mut q_enemies: Query<(&mut UckoState, &Transform), With<Ucko>>,
) {
    const QUARTERSCREEN: f32 = WINDOW_WIDTH / 8.0;
    let camera_position = camera.translation.x;

    q_enemies
        .iter_mut()
        .filter(|(state, ..)| matches!(**state, UckoState::Sprint))
        .for_each(|(mut state, transform)| {
            let distance_to_center = camera_position - transform.translation.x;
            if distance_to_center < QUARTERSCREEN {
                *state = UckoState::Chase;
            }
        });
}

fn enemy_jump(
    ground_bodies: Query<(), With<TiledColliderMarker>>,
    mut q_enemies: Query<(&mut UckoState, &RayHits, &mut LinearVelocity), With<Ucko>>,
) {
    const JUMP_IMPULSE: Vec2 = vec2(16.0, 256.0);

    q_enemies
        .iter_mut()
        .filter(|(state, ..)| matches!(**state, UckoState::Chase | UckoState::Sprint))
        .filter(|(_state, ray_hits, ..)| {
            ray_hits
                .iter()
                .filter_map(|hit| ground_bodies.contains(hit.entity).then_some(hit.normal))
                .any(|normal| normal.y.abs() < 0.001) // normal is near horizontal
        })
        .for_each(|(mut state, _ray_hits, mut velocity)| {
            velocity.0 += JUMP_IMPULSE;
            *state = UckoState::Jump;
        });
}

fn enemy_land_jump(
    mut q_enemies: Query<(&mut UckoState, &Grounded), (With<Ucko>, Changed<Grounded>)>,
) {
    q_enemies
        .iter_mut()
        .filter(|(state, grounded)| grounded.0 && matches!(**state, UckoState::Jump))
        .for_each(|(mut state, ..)| *state = UckoState::Chase);
}

fn enemy_dive(
    player: Single<Entity, With<Player>>,
    mut q_enemies: Query<(&mut UckoState, &RayHits, &mut LinearVelocity), With<Ucko>>,
) {
    const DIVE_IMPULSE: Vec2 = vec2(128.0, 64.0);

    q_enemies
        .iter_mut()
        .filter(|(state, ..)| matches!(**state, UckoState::Chase | UckoState::Sprint))
        .filter(|(_state, ray_hits, ..)| ray_hits.iter().any(|hit| hit.entity == *player))
        .for_each(|(mut state, _ray_hits, mut velocity)| {
            velocity.0 += DIVE_IMPULSE;
            *state = UckoState::Dive;
        });
}

fn enemy_land_dive(
    mut q_enemies: Query<
        (&mut UckoState, &Grounded, &mut LinearVelocity),
        (With<Ucko>, Changed<Grounded>),
    >,
) {
    q_enemies
        .iter_mut()
        .filter(|(state, grounded, ..)| matches!(**state, UckoState::Dive) && grounded.0)
        .for_each(|(mut state, _grounded, mut velocity)| {
            velocity.set_if_neq(LinearVelocity::ZERO);
            *state = UckoState::Recover
        });
}

fn enemy_recover(
    mut q_enemies: Query<(&mut UckoState, &mut RecoveryTimer), With<Ucko>>,
    time: Res<Time>,
) {
    q_enemies
        .iter_mut()
        .filter(|(state, _timer)| matches!(**state, UckoState::Recover))
        .for_each(|(mut state, mut timer)| {
            if timer.tick(time.delta()).just_finished() {
                timer.reset();
                *state = UckoState::Chase
            }
        });
}

fn conclude_bones(mut commands: Commands) {
    commands.insert_resource(BonesTimer::default());
    commands.insert_resource(Gravity::ZERO);
    commands.set_state(GameState::TopDown);
}
