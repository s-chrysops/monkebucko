#![allow(clippy::type_complexity)]
use std::{f32::consts::FRAC_PI_2, time::Duration};

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::*;
use bevy_rand::prelude::*;

use crate::{
    RENDER_LAYER_WORLD, WINDOW_HEIGHT, WINDOW_WIDTH, animation::SpriteAnimation, despawn_screen,
    game::effects::*,
};

use super::*;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, SubStates)]
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
            setup_enemies,
            setup_sprite_effects,
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
                player_jump.run_if(
                    in_state(BonesState::Playing).and(just_pressed_jump.and(player_grounded)),
                ),
                update_player_state,
                update_player_animation,
            )
                .chain()
                .run_if(in_state(MovementEnabled)),
            enemy_sprint,
            enemy_chase,
            enemy_jump,
            enemy_dive,
            enemy_land,
            enemy_recover,
            update_enemy_animation,
            enemy_fire,
        )
            .run_if(in_state(BonesState::Playing)),
    )
    .add_systems(
        FixedUpdate,
        (
            update_enemies_velocity,
            update_player_velocity.run_if(player_grounded),
        )
            .run_if(in_state(BonesState::Playing)),
    )
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
    .init_resource::<BonesTimer>()
    .register_type::<BonesHealth>()
    .register_type::<Grounded>()
    .register_type::<UckoState>();
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
    mut asset_tracker: ResMut<AssetTracker>,
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
}

#[derive(Debug, Component, Deref, DerefMut, Reflect)]
#[reflect(Component)]
struct BonesHealth(u8);

const PLAYER_MAX_HEALTH: u8 = 8;

// #[derive(Debug, Component, Deref, DerefMut, PartialEq, Reflect)]
// #[reflect(Component)]
// struct OldGrounded(bool);

#[derive(Debug, Component, Default, Deref, DerefMut, PartialEq, Reflect)]
#[reflect(Component)]
struct Grounded {
    forward: Option<Vec2>,
}

#[derive(Debug, Component, Default, PartialEq, Reflect)]
#[reflect(Component)]
enum PlayerState {
    #[default]
    Run,
    Still,
    Jump,
}

fn setup_player(
    mut commands: Commands,
    mut asset_tracker: ResMut<AssetTracker>,
    asset_server: Res<AssetServer>,
) {
    const PLAYER_START: Vec3 = vec3(384.0, 180.0, 1.0);

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
            PlayerState::default(),
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
                LinearDamping(1.0),
                Grounded::default(),
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

#[derive(Debug, Component, Deref, DerefMut, Reflect)]
#[reflect(Component)]
struct FireTimer(Timer);

impl Default for FireTimer {
    fn default() -> Self {
        const FIRE_TIME: f32 = 8.0;
        FireTimer(Timer::from_seconds(FIRE_TIME, TimerMode::Once))
    }
}

#[derive(Debug, Component)]
struct DamageCollider;

const ENEMY_AMOUNT: u8 = 3;

fn setup_enemies(
    mut commands: Commands,
    mut asset_tracker: ResMut<AssetTracker>,
    asset_server: Res<AssetServer>,
    mut rng: GlobalEntropy<WyRand>,
) {
    const ENEMY_START: Vec3 = vec3(64.0, 180.0, 1.0);
    const ENEMY_SPACING: f32 = 48.0;

    let enemy_image = asset_server.load("sprites/ucko/crawl.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(64), 8, 1, None, None);
    let layout = asset_server.add(layout);
    asset_tracker.push(enemy_image.clone().untyped());

    let enemy_sprite = Sprite {
        image: enemy_image,
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
        let offset_x = Vec3::NEG_X * ENEMY_SPACING * i as f32;
        let offset_z = Vec3::NEG_Z * 0.01 * i as f32;
        commands.spawn((
            OnBones,
            Ucko,
            Name::new(format!("Ucko_{}", i)),
            Transform::from_translation(ENEMY_START + offset_x + offset_z),
            (
                // Ucko
                UckoState::default(),
                FireTimer::default(),
                RecoveryTimer::default(),
                rng.fork_rng(),
            ),
            (
                // Visual
                enemy_sprite.clone(),
                SpriteAnimation::new(0, 7, 12).looping(),
                Visibility::default(),
                RENDER_LAYER_WORLD,
            ),
            (
                // Physics
                RigidBody::Dynamic,
                Collider::capsule_endpoints(16.0, vec2(0.0, -8.0), vec2(0.0, -16.0)),
                RayCaster::new(Vec2::ZERO, Dir2::X)
                    .with_max_distance(64.0)
                    .with_query_filter(raycaster_filter.clone()),
                enemy_collision_layers,
                LockedAxes::ROTATION_LOCKED,
                LinearVelocity(Vec2::X * 36.0),
                Friction::ZERO,
                Grounded::default(),
                children![(
                    DamageCollider,
                    Sensor,
                    Collider::rectangle(54.0, 30.0),
                    Transform::from_xyz(0.0, -17.0, 0.0),
                    hitbox_collision_layers,
                )],
            ),
        ));
    });
}

#[derive(Debug, Resource)]
struct BonesEffects {
    fireball: Sprite,
}

fn setup_sprite_effects(
    mut commands: Commands,
    mut asset_tracker: ResMut<AssetTracker>,
    asset_server: Res<AssetServer>,
) {
    let fireball_image = asset_server.load("sprites/effects/fireball.png");
    asset_tracker.push(fireball_image.clone().untyped());

    let fireball_layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 6, 1, None, None);
    let fireball_layout = asset_server.add(fireball_layout);
    let fireball_sprite = Sprite::from_atlas_image(fireball_image, fireball_layout.into());

    commands.insert_resource(BonesEffects {
        fireball: fireball_sprite,
    });
}

fn wait_till_loaded(
    mut asset_tracker: ResMut<AssetTracker>,
    asset_server: Res<AssetServer>,
    mut gravity: ResMut<Gravity>,
    mut bones_state: ResMut<NextState<BonesState>>,
) {
    if asset_tracker.is_ready(asset_server) {
        *gravity = Gravity(Vec2::NEG_Y * 9.81 * 32.0);
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
    q_hitboxes: Query<(), With<DamageCollider>>,
    mut player_health: Single<&mut BonesHealth, With<Player>>,
) {
    if q_hitboxes.contains(trigger.collider) {
        player_health.0 = player_health.saturating_sub(1);
    }
}

fn update_buckos_grounded(
    collisions: Collisions,
    ground_bodies: Query<(), With<TiledColliderMarker>>,
    mut buckos: Query<(Entity, &mut Grounded)>,
) {
    buckos.iter_mut().for_each(|(entity, mut grounded)| {
        let is_grounded = collisions.collisions_with(entity).find_map(|contact| {
            let is_first = entity == contact.collider1;

            let other_body = match is_first {
                true => contact.body2,
                false => contact.body1,
            };

            if !ground_bodies.contains(other_body?) {
                return None;
            }

            contact.manifolds.iter().find_map(|manifold| {
                let normal = match is_first {
                    true => -manifold.normal,
                    false => manifold.normal,
                }; // Normal points from ground to bucko

                (normal.y.abs() > 0.001).then_some(-normal.perp())
            }) // contact isn't completely vertical, you're grounded bucko
        });

        grounded.set_if_neq(Grounded {
            forward: is_grounded,
        });
    })
}

fn player_grounded(player_grounded: Single<&Grounded, With<Player>>) -> bool {
    player_grounded.is_some()
}

fn update_player_velocity(
    player: Single<(&BonesHealth, &Grounded, &mut LinearVelocity), With<Player>>,
    user_input: Res<UserInput>,
    time: Res<Time<Fixed>>,
) {
    const ACCELERATION: f32 = 256.0;

    let (health, grounded, mut velocity) = player.into_inner();

    let max_x_velocity = (health.0 as f32 * 2.0) + 32.0;
    let forward = grounded
        .forward
        .expect("System should only run if grounded");

    velocity.0 += forward * user_input.raw_vector.x * ACCELERATION * time.delta_secs();
    velocity.x = velocity.x.clamp(0.0, max_x_velocity);
}

fn player_jump(mut player_velocity: Single<&mut LinearVelocity, With<Player>>) {
    const JUMP_IMPULSE: Vec2 = vec2(48.0, 256.0);
    player_velocity.0 += JUMP_IMPULSE;
}

fn update_player_state(
    player: Single<(&Grounded, &LinearVelocity, &mut PlayerState), With<Player>>,
) {
    const RUN_THRESHOLD: f32 = 4.0;

    let (grounded, velocity, mut state) = player.into_inner();

    let new_state = match grounded.is_some() {
        true => match velocity.x > RUN_THRESHOLD {
            true => PlayerState::Run,
            false => PlayerState::Still,
        },
        false => PlayerState::Jump,
    };

    state.set_if_neq(new_state);
}

fn update_player_animation(
    player: Single<
        (
            Ref<BonesHealth>,
            Ref<PlayerState>,
            &LinearVelocity,
            &mut SpriteAnimation,
        ),
        With<Player>,
    >,
) {
    const ANIMATION_ROWS: u8 = 7;
    const SPRITES_PER_ROW: usize = 8;
    const STILLS_INDEX: usize = 56;
    const MIN_FPS: u8 = 6;

    let (health, state, velocity, mut animation) = player.into_inner();

    let current_row = (PLAYER_MAX_HEALTH - health.0).min(ANIMATION_ROWS - 1) as usize;
    let new_index = SPRITES_PER_ROW * current_row;

    if state.is_changed() || health.is_changed() {
        *animation = match *state {
            PlayerState::Run => SpriteAnimation::new(new_index, new_index + 7, 12).looping(),
            PlayerState::Jump => SpriteAnimation::set_frame(new_index), // first frame of row is jump
            PlayerState::Still => SpriteAnimation::set_frame(STILLS_INDEX + current_row),
        };

        if matches!(*state, PlayerState::Run) {
            animation
                .as_mut()
                .change_fps(MIN_FPS + (velocity.x / 8.0) as u8);
        }
    }
}

fn update_enemies_velocity(
    mut q_enemies: Query<(&UckoState, &Grounded, &mut LinearVelocity), With<Ucko>>,
    time_fixed: Res<Time<Fixed>>,
) {
    const MAX_X_VELOCITY_NORMAL: f32 = 33.0;
    const MAX_X_VELOCITY_SPRINT: f32 = 48.0;
    const ACCELERATION: f32 = 256.0;

    q_enemies
        .iter_mut()
        .filter(|(state, grounded, ..)| {
            grounded.is_some() && matches!(**state, UckoState::Chase | UckoState::Sprint)
        })
        .for_each(|(state, grounded, mut velocity)| {
            let forward = grounded.forward.unwrap();
            velocity.0 += forward * ACCELERATION * time_fixed.delta_secs();
            velocity.x = match state {
                UckoState::Chase => velocity.x.clamp(0.0, MAX_X_VELOCITY_NORMAL),
                UckoState::Sprint => velocity.x.clamp(0.0, MAX_X_VELOCITY_SPRINT),
                _ => unreachable!(),
            }
        });
}

fn update_enemy_animation(
    mut q_enemies: Query<(&UckoState, &mut SpriteAnimation), (With<Ucko>, Changed<UckoState>)>,
) {
    q_enemies.iter_mut().for_each(|(state, mut animation)| {
        *animation = match *state {
            UckoState::Chase => SpriteAnimation::new(0, 7, 12).looping(),
            UckoState::Sprint => SpriteAnimation::new(0, 7, 16).looping(),
            _ => SpriteAnimation::set_frame(0),
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
    mut q_enemies: Query<(&mut UckoState, &Transform, &mut Entropy<WyRand>), With<Ucko>>,
    mut stop_offset: Local<f32>,
) {
    const QUARTERSCREEN: f32 = WINDOW_WIDTH / 8.0;
    let camera_position = camera.translation.x;

    q_enemies
        .iter_mut()
        .filter(|(state, ..)| matches!(**state, UckoState::Sprint))
        .for_each(|(mut state, transform, mut rng)| {
            let distance_to_center = camera_position - transform.translation.x;
            if distance_to_center < QUARTERSCREEN + *stop_offset {
                *stop_offset = random_range(&mut rng, -64.0, 64.0);
                *state = UckoState::Chase;
            }
        });
}

fn enemy_jump(
    ground_bodies: Query<(), With<TiledColliderMarker>>,
    mut q_enemies: Query<(&mut UckoState, &RayHits, &mut LinearVelocity), With<Ucko>>,
) {
    const JUMP_IMPULSE: Vec2 = vec2(48.0, 256.0);

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
            velocity.0 = JUMP_IMPULSE;
            *state = UckoState::Jump;
        });
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

fn enemy_land(
    mut q_enemies: Query<
        (&mut UckoState, &Grounded, &mut LinearVelocity),
        (With<Ucko>, Changed<Grounded>),
    >,
) {
    q_enemies
        .iter_mut()
        .filter(|(state, grounded, ..)| {
            grounded.is_some() && matches!(**state, UckoState::Dive | UckoState::Jump)
        })
        .for_each(|(mut state, _grounded, mut velocity)| {
            *state = match *state {
                UckoState::Dive => {
                    velocity.set_if_neq(LinearVelocity::ZERO);
                    UckoState::Recover
                }
                UckoState::Jump => UckoState::Chase,
                _ => unreachable!(),
            }
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

fn enemy_fire(
    mut commands: Commands,
    mut q_enemies: Query<
        (&UckoState, &mut FireTimer, &mut Entropy<WyRand>, &Transform),
        With<Ucko>,
    >,
    time: Res<Time>,
) {
    q_enemies
        .iter_mut()
        .filter(|(state, ..)| matches!(**state, UckoState::Chase))
        .for_each(|(_state, mut timer, mut rng, transform)| {
            if timer.tick(time.delta()).just_finished() {
                timer.reset();
                let headstart = random_range(&mut rng, 0.0, 4.0);
                timer.set_elapsed(Duration::from_secs_f32(headstart));
                if rng_percentage(&mut rng, 0.5) {
                    commands.run_system_cached_with(spawn_fireball, transform.translation);
                }
            }
        });
}

#[derive(Debug, Component)]
struct Fireball;

#[derive(Debug, Component)]
struct FireballCollider;

fn spawn_fireball(In(origin): In<Vec3>, mut commands: Commands, effects: Res<BonesEffects>) {
    const FIREBALL_VELOCITY: Vec2 = vec2(512.0, 256.0);
    // info!("Fireball spawned at {}", origin.truncate());

    let fireball_collision_layers = CollisionLayers::new(
        [ColliderLayer::Enemy],
        [ColliderLayer::Default, ColliderLayer::World],
    );

    let hitbox_collision_layers = CollisionLayers::new(
        [ColliderLayer::Enemy],
        [ColliderLayer::Default, ColliderLayer::Player],
    );

    let fireball_entity = commands
        .spawn((
            Fireball,
            Name::new("Fireball"),
            effects.fireball.clone(),
            SpriteAnimation::new(0, 5, 12).looping(),
            RigidBody::Dynamic,
            AngularVelocity(-0.4),
            LinearVelocity(FIREBALL_VELOCITY),
            LinearDamping(1.0),
            Transform::from_translation(origin).with_rotation(Quat::from_rotation_z(FRAC_PI_2)),
            Visibility::Visible,
            children![(
                DamageCollider,
                Sensor,
                Collider::circle(6.0),
                hitbox_collision_layers,
                Transform::from_xyz(0.0, -9.0, 0.0)
            )],
        ))
        .id();

    commands
        .spawn((
            FireballCollider,
            ChildOf(fireball_entity),
            Collider::circle(6.0),
            CollisionEventsEnabled,
            fireball_collision_layers,
            Transform::from_xyz(0.0, -9.0, 0.0),
        ))
        .observe(fireball_land);
}

fn fireball_land(
    trigger: Trigger<OnCollisionStart>,
    q_fireball_colliders: Query<&ChildOf, With<FireballCollider>>,
    mut commands: Commands,
) {
    if let Ok(ChildOf(entity)) = q_fireball_colliders.get(trigger.target()) {
        commands.entity(*entity).despawn();
    }
}

fn conclude_bones(mut commands: Commands) {
    commands.insert_resource(BonesTimer::default());
    commands.insert_resource(Gravity::ZERO);
    commands.set_state(GameState::TopDown);
}
