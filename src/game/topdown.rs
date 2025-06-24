#![allow(clippy::type_complexity)]
use avian2d::prelude::*;
use bevy::{prelude::*, time::Stopwatch};
use bevy_ecs_tiled::prelude::*;

use super::*;
use crate::{RENDER_LAYER_WORLD, WINDOW_HEIGHT, WINDOW_WIDTH, animation::*};

#[derive(Debug, Component)]
struct OnTopDown;

pub fn topdown_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::TopDown), (spawn_player, topdown_setup))
        .add_systems(
            Update,
            set_player_hop
                .run_if(in_state(GameState::TopDown))
                .run_if(in_state(MovementState::Enabled)),
        )
        .add_systems(
            Update,
            (camera_system, update_player_z).run_if(in_state(GameState::TopDown)),
        )
        .add_observer(setup_current_map)
        .register_type::<MoveInput>()
        .register_type::<Warp>()
        .init_resource::<CurrentMap>()
        .init_resource::<TopdownMapHandles>();
}

fn topdown_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    topdown_maps: Res<TopdownMapHandles>,
    current_map: ResMut<CurrentMap>,
) {
    use crate::auto_scaling::AspectRatio;
    use bevy::render::camera::ScalingMode;

    commands.spawn((
        WorldCamera,
        Camera2d,
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Transform::from_scale(Vec3::splat(0.5)),
        AspectRatio(16.0 / 9.0),
        Projection::from({
            OrthographicProjection {
                near: -1000.0,
                scaling_mode: ScalingMode::Fixed {
                    width:  WINDOW_WIDTH as f32,
                    height: WINDOW_HEIGHT as f32,
                },
                ..OrthographicProjection::default_3d()
            }
        }),
        RENDER_LAYER_WORLD,
    ));

    commands.spawn((
        OnTopDown,
        Transform::from_xyz(500.0, 500.0, 0.0),
        Visibility::default(),
        Sprite::from_image(asset_server.load("bucko.png")),
        RigidBody::Static,
        Collider::circle(32.0),
        RENDER_LAYER_WORLD,
    ));

    commands
        .spawn(TiledMapHandle(topdown_maps[current_map.index].clone_weak()))
        .observe(setup_collider_bodies);
}

#[derive(Debug, Default, Resource)]
struct CurrentMap {
    index: TopdownMapIndex,
    rect:  Rect,
}

const Z_BETWEEN_LAYERS: f32 = 100.0;

fn setup_current_map(
    trigger: Trigger<TiledMapCreated>,
    q_tiled_maps: Query<&mut TiledMapStorage, With<TiledMapMarker>>,
    mut q_tiled_objects: Query<&mut Transform, With<TiledMapObject>>,
    a_tiled_maps: Res<Assets<TiledMap>>,
    mut current_map: ResMut<CurrentMap>,
) {
    let Some(tiled_map) = trigger.event().get_map_asset(&a_tiled_maps) else {
        warn!("Failed to load Tiled map asset");
        return;
    };

    // let tilemap_size = tiled_map.tilemap_size;
    // info!("Map Size = x: {}, y: {}", tilemap_size.x, tilemap_size.y);
    current_map.rect = tiled_map.rect;

    let Ok(map_storage) = q_tiled_maps.get(trigger.entity) else {
        warn!("Failed to load Tiled map storage");
        return;
    };

    // Objects higher up on the map will be given a greater negative z-offset
    map_storage.objects.iter().for_each(|(_tiled_id, entity)| {
        if let Ok(mut transform) = q_tiled_objects.get_mut(*entity) {
            let offset_y = transform.translation.y - tiled_map.rect.min.y;
            transform.translation.z -= offset_y / tiled_map.rect.height() * Z_BETWEEN_LAYERS;
        }
    });
}

fn setup_collider_bodies(
    trigger: Trigger<TiledColliderCreated>,
    mut commands: Commands,
    q_tiled_objects: Query<Option<&Warp>, With<TiledMapObject>>,
    q_tiled_colliders: Query<&ChildOf, With<TiledColliderMarker>>,
) {
    let ChildOf(parent) = q_tiled_colliders
        .get(trigger.entity)
        .expect("Failed to get collider parent");

    let warp = q_tiled_objects
        .get(*parent)
        .expect("Failed to get object from query");

    if warp.is_some() {
        commands
            .entity(trigger.entity)
            .insert((Sensor, CollisionEventsEnabled))
            .observe(warp_player);
    }

    commands.entity(trigger.entity).insert(RigidBody::Static);
}

const TOTAL_TOPDOWN_MAPS: usize = 7;

#[derive(Clone, Copy, Debug, Default, Reflect)]
#[reflect(Default)]
enum TopdownMapIndex {
    Mountain,
    Backyard,
    Fields,
    Forest,
    #[default]
    Buckotown,
    Farm,
    Beach,
}

#[derive(Debug, Deref, Resource)]
struct TopdownMapHandles([Handle<TiledMap>; TOTAL_TOPDOWN_MAPS]);

impl std::ops::Index<TopdownMapIndex> for [Handle<TiledMap>] {
    type Output = Handle<TiledMap>;

    fn index(&self, index: TopdownMapIndex) -> &Self::Output {
        &self[index as usize]
    }
}

impl FromWorld for TopdownMapHandles {
    fn from_world(world: &mut World) -> Self {
        const MAP_PATHS: [&str; TOTAL_TOPDOWN_MAPS] = [
            "maps/mountains.tmx",
            "maps/backyard.tmx",
            "maps/fields.tmx",
            "maps/forest.tmx",
            "maps/buckotown.tmx",
            "maps/farm.tmx",
            "maps/beach.tmx",
        ];

        TopdownMapHandles(
            MAP_PATHS.map(|path| world.resource::<AssetServer>().load::<TiledMap>(path)),
        )
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
struct Warp {
    target_map:      TopdownMapIndex,
    point_mode:      bool,
    offset_or_point: Vec2,
}

#[allow(clippy::too_many_arguments)]
fn warp_player(
    trigger: Trigger<OnCollisionStart>,
    q_tiled_colliders: Query<&ChildOf, With<TiledColliderMarker>>,
    q_tiled_objects: Query<&Warp, With<TiledMapObject>>,
    tiled_map: Single<Entity, With<TiledMapMarker>>,
    topdown_maps: Res<TopdownMapHandles>,
    mut current_map: ResMut<CurrentMap>,
    player: Single<(Entity, &mut Transform), (With<Player>, Without<WorldCamera>)>,
    camera: Single<&mut Transform, With<WorldCamera>>,
    mut commands: Commands,
) {
    let ChildOf(parent) = q_tiled_colliders
        .get(trigger.target())
        .expect("Failed to get collider parent");

    let warp = q_tiled_objects
        .get(*parent)
        .expect("Failed to get object from query");

    let (player_entity, mut player_transform) = player.into_inner();

    if trigger.collider != player_entity {
        warn!("wtf");
        return;
    }
    current_map.index = warp.target_map;

    let Vec3 { x, y, z } = player_transform.translation;
    info!("Player at ({}, {}, {})", x, y, z);

    match warp.point_mode {
        true => player_transform.translation = warp.offset_or_point.extend(0.0),
        false => player_transform.translation += warp.offset_or_point.extend(0.0),
    };

    camera.into_inner().translation = player_transform.translation;

    let Vec3 { x, y, z } = player_transform.translation;
    info!(
        "Warping player to {:?} ({}, {}, {})",
        warp.target_map, x, y, z
    );

    commands.entity(*tiled_map).despawn();
    commands
        .spawn(TiledMapHandle(topdown_maps[warp.target_map].clone_weak()))
        .observe(setup_collider_bodies);
}

fn spawn_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let player_sprites: Handle<Image> = asset_server.load("sprites/bucko_bounce.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 8, 2, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    commands
        .spawn((
            OnTopDown,
            Player,
            InteractTarget(None),
            //
            Transform::default(),
            Visibility::default(),
            //
            Sprite::from_atlas_image(
                player_sprites,
                TextureAtlas {
                    layout: texture_atlas_layout,
                    index:  0,
                },
            ),
            MoveInput::default(),
            HopState::Idle,
            SpriteAnimation::set_frame(0),
            //
            RigidBody::Dynamic,
            Collider::circle(14.0),
            LockedAxes::ROTATION_LOCKED,
            LinearVelocity::ZERO,
            LinearDamping(3.0),
            //
            RENDER_LAYER_WORLD,
        ))
        .observe(update_player_hop)
        .observe(cycle_hop_animations);
}

fn update_player_z(
    player: Single<&mut Transform, (With<Player>, Changed<Transform>)>,
    current_map: Res<CurrentMap>,
) {
    if current_map.rect == Rect::default() {
        return;
    }
    let mut transform = player.into_inner();
    let offset_y = transform.translation.y - current_map.rect.min.y;
    transform.translation.z =
        -offset_y / current_map.rect.height() * Z_BETWEEN_LAYERS - Z_BETWEEN_LAYERS;
}

#[derive(Debug, Event)]
struct HopUpdate;

#[derive(Debug, Default, Component, Clone, Copy, Reflect)]
enum HopState {
    Ready,
    Charging,
    Airborne,
    Landing,
    #[default]
    Idle,
}

impl HopState {
    fn cycle_state(&mut self) {
        *self = match self {
            HopState::Ready => HopState::Charging,
            HopState::Charging => HopState::Airborne,
            HopState::Airborne => HopState::Landing,
            HopState::Landing => HopState::Idle,
            HopState::Idle => HopState::Idle,
        };
    }
}

#[derive(Debug, Component, Clone, Copy, Reflect)]
#[reflect(Component)]
struct MoveInput {
    input_vector:   Vec2,
    last_direction: Dir2,
}

impl Default for MoveInput {
    fn default() -> Self {
        MoveInput {
            input_vector:   Vec2::ZERO,
            last_direction: Dir2::EAST,
        }
    }
}

fn set_player_hop(
    mut commands: Commands,
    settings: Res<Persistent<Settings>>,
    key_input: Res<ButtonInput<KeyCode>>,
    player: Single<(Entity, &mut MoveInput, &mut HopState), With<Player>>,
) {
    let mut input_vector = Vec2::ZERO;
    if key_input.pressed(settings.up) {
        input_vector += Vec2::Y;
    }
    if key_input.pressed(settings.down) {
        input_vector -= Vec2::Y;
    }
    if key_input.pressed(settings.right) {
        input_vector += Vec2::X;
    }
    if key_input.pressed(settings.left) {
        input_vector -= Vec2::X;
    }

    let (entity, mut move_input, mut hop_state) = player.into_inner();
    move_input.input_vector = input_vector;
    if let Ok(new_direction) = Dir2::new(input_vector) {
        move_input.last_direction = new_direction;
        *hop_state = HopState::Ready;
        commands.trigger_targets(HopUpdate, entity);
    }
}

fn cycle_hop_animations(
    trigger: Trigger<SpriteAnimationFinished>,
    mut commands: Commands,
    player: Single<&mut HopState, With<Player>>,
) {
    let mut hop_state = player.into_inner();
    hop_state.cycle_state();
    commands.trigger_targets(HopUpdate, trigger.target());
}

fn update_player_hop(
    _trigger: Trigger<HopUpdate>,
    mut movement_state: ResMut<NextState<MovementState>>,
    player: Single<
        (
            &MoveInput,
            &HopState,
            &mut SpriteAnimation,
            &mut LinearVelocity,
        ),
        With<Player>,
    >,
) {
    const HOP_IMPULSE: f32 = 128.0;
    const SPRITES_PER_ROW: usize = 8;

    let (move_input, hop_state, mut animation, mut velocity) = player.into_inner();

    // Map direction to spritesheet y-offset
    // x.signum() will suffice whilst only right/left sprites are present
    let index_offset = SPRITES_PER_ROW
        * match move_input.last_direction.x.signum() {
            // Facing right
            1.0 => 0,
            // Facing left
            -1.0 => 1,
            // Up or down, default to right
            _ => 0,
        };

    match hop_state {
        HopState::Ready => {
            movement_state.set(MovementState::Disabled);
            *animation = SpriteAnimation::set_frame(index_offset);
        }
        HopState::Charging => {
            *animation = SpriteAnimation::new(1 + index_offset, 2 + index_offset, 12);
        }
        HopState::Airborne => {
            *velocity = LinearVelocity(move_input.last_direction * HOP_IMPULSE);
            *animation = SpriteAnimation::new(3 + index_offset, 5 + index_offset, 12);
        }
        HopState::Landing => {
            *animation = SpriteAnimation::new(6 + index_offset, 7 + index_offset, 12);
        }
        HopState::Idle => {
            movement_state.set(MovementState::Enabled);
            *animation = SpriteAnimation::set_frame(index_offset);
        }
    }
}

fn camera_system(
    mut camera_transform: Single<&mut Transform, With<WorldCamera>>,
    player: Single<(&Transform, &MoveInput), (With<Player>, Without<WorldCamera>)>,
    current_map: Res<CurrentMap>,
    mut stopwatch: Local<Stopwatch>,
    mut target_direction: Local<Vec2>,
    time: Res<Time>,
) {
    const WINDOW_SIZE: Vec2 = Vec2::new(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32);
    // Needed for correction caused by origin being in the center of the bottom-left-most tile
    const HALF_TILE_SIZE: f32 = 16.0;
    // Percentage of the view size the camera will target
    const LOOKAHEAD: f32 = 0.4;
    // Percentage of distance the camera will move to the target per render frame
    const SMOOTH: f32 = 0.01;
    const NEW_DIRECTION_DELAY: f32 = 1.0;

    let (player_transform, move_input) = player.into_inner();
    let view_size = WINDOW_SIZE * camera_transform.scale.xy();

    if *target_direction != move_input.input_vector
        && stopwatch.tick(time.delta()).elapsed_secs() > NEW_DIRECTION_DELAY
    {
        stopwatch.reset();
        *target_direction = move_input.input_vector;
    }

    let camera_offset = (view_size * LOOKAHEAD * *target_direction).extend(0.0);
    let camera_target = player_transform.translation + camera_offset;

    // Bounds camera from rendering the void outside the current map
    let camera_min = (current_map.rect.min + (view_size / 2.0) - HALF_TILE_SIZE).extend(f32::MIN);
    let camera_max = (current_map.rect.max - (view_size / 2.0) - HALF_TILE_SIZE).extend(f32::MAX);

    camera_transform.translation = camera_transform
        .translation
        .lerp(camera_target, SMOOTH)
        .clamp(camera_min, camera_max);
}
