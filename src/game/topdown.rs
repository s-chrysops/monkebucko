#![allow(clippy::type_complexity)]
use avian2d::prelude::*;
use bevy::{
    animation::{AnimationTarget, animated_field},
    prelude::*,
    time::Stopwatch,
};
use bevy_ecs_tiled::prelude::*;
use bevy_ecs_tilemap::tiles::TileStorage;

use super::*;
use crate::{RENDER_LAYER_WORLD, WINDOW_HEIGHT, WINDOW_WIDTH, animation::*};

#[derive(Debug, Component)]
struct OnTopDown;

pub fn topdown_plugin(app: &mut App) {
    app.add_systems(
        OnTransition {
            exited:  GameState::Egg,
            entered: GameState::TopDown,
        },
        fade_from_egg,
    )
    .add_systems(OnEnter(GameState::TopDown), (spawn_player, topdown_setup))
    .add_systems(
        Update,
        (
            camera_system,
            update_player_submerged,
            update_player_z,
            wait_for_fade,
        )
            .run_if(in_state(GameState::TopDown))
            .run_if(|current_map: Res<CurrentMap>| current_map.loaded),
    )
    .add_systems(
        Update,
        player_hop
            .run_if(in_state(GameState::TopDown))
            .run_if(in_state(MovementState::Enabled))
            .run_if(not(player_submerged)),
    )
    .add_systems(
        Update,
        player_swim
            .run_if(in_state(GameState::TopDown))
            .run_if(in_state(MovementState::Enabled))
            .run_if(player_submerged),
    )
    .add_observer(setup_current_map)
    .register_type::<HopState>()
    .register_type::<Submerged>()
    .register_type::<WaterTile>()
    .register_type::<Warp>()
    .init_resource::<CurrentMap>()
    .init_resource::<LastPlayerLocation>()
    .init_resource::<TopdownMapHandles>();
}

fn topdown_setup(
    mut commands: Commands,
    // asset_server: Res<AssetServer>,
    last_player_location: Res<LastPlayerLocation>,
    topdown_maps: Res<TopdownMapHandles>,
    current_map: Res<CurrentMap>,
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
        Transform::from_translation(last_player_location.0).with_scale(Vec3::splat(0.5)),
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

    commands
        .spawn(TiledMapHandle(topdown_maps[current_map.index].clone_weak()))
        .observe(setup_collider_bodies);
}

#[derive(Debug, Default, Resource)]
struct CurrentMap {
    loaded: bool,
    index:  TopdownMapIndex,

    rect:         Rect,
    tilemap_size: TilemapSize,

    entity:      Option<Entity>,
    water_layer: Option<Entity>,
}

const Z_BETWEEN_LAYERS: f32 = 100.0;

fn setup_current_map(
    trigger: Trigger<TiledMapCreated>,
    a_tiled_maps: Res<Assets<TiledMap>>,
    q_tiled_maps: Query<(Entity, &mut TiledMapStorage), With<TiledMapMarker>>,
    q_tiled_layers: Query<(Entity, &Name), With<TiledMapTileLayerForTileset>>,
    mut q_tiled_objects: Query<&mut Transform, With<TiledMapObject>>,
    mut current_map: ResMut<CurrentMap>,
) {
    let Some(tiled_map) = trigger.event().get_map_asset(&a_tiled_maps) else {
        warn!("Failed to load Tiled map asset");
        return;
    };

    let Ok((map_entity, map_storage)) = q_tiled_maps.get(trigger.entity) else {
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

    current_map.loaded = true;

    current_map.rect = tiled_map.rect;
    current_map.tilemap_size = tiled_map.tilemap_size;

    current_map.entity = Some(map_entity);
    current_map.water_layer = q_tiled_layers.iter().find_map(|(entity, name)| {
        match name.as_str() == "TiledMapTileLayerForTileset(water, water)" {
            true => Some(entity),
            false => None,
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

fn fade_from_egg(
    mut commands: Commands,
    fade_white: Single<(Entity, &AnimationTarget), (With<Fade>, Without<AnimationPlayer>)>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut animation_clips: ResMut<Assets<AnimationClip>>,
) {
    let (fade_entity, fade_target) = fade_white.into_inner();
    let (animation_graph, animation_node) = AnimationGraph::from_clip(animation_clips.add({
        let mut fade_in_clip = AnimationClip::default();
        fade_in_clip.add_curve_to_target(
            fade_target.id,
            AnimatableCurve::new(
                animated_field!(Opacity::0),
                EasingCurve::new(1.0, 0.0, EaseFunction::ExponentialInOut),
            ),
        );
        fade_in_clip.add_event_fn(1.0, |commands, entity, _time, _weight| {
            commands.entity(entity).despawn();
        });
        fade_in_clip
    }));
    let animation_graph_handle = animation_graphs.add(animation_graph);
    let mut animation_player = AnimationPlayer::default();
    animation_player.play(animation_node);

    commands.entity(fade_entity).insert((
        animation_player,
        AnimationGraphHandle(animation_graph_handle),
        AnimationTarget {
            id:     fade_target.id,
            player: fade_entity,
        },
    ));
}

fn wait_for_fade(
    q_fade: Query<&mut AnimationPlayer, With<Fade>>,
    mut movement_state: ResMut<NextState<MovementState>>,
) {
    if q_fade.iter().all(|player| player.all_finished()) {
        movement_state.set(MovementState::Enabled);
    }
}

const TOTAL_TOPDOWN_MAPS: usize = 7;

#[derive(Clone, Copy, Debug, Default, Reflect)]
#[reflect(Default)]
enum TopdownMapIndex {
    #[default]
    Mountain,
    Backyard,
    Fields,
    Forest,
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
    mut camera: Single<&mut Transform, With<WorldCamera>>,
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

    *current_map = CurrentMap {
        index: warp.target_map,
        ..default()
    };

    let Vec3 { x, y, z } = player_transform.translation;
    info!("Player at ({}, {}, {})", x, y, z);

    match warp.point_mode {
        true => player_transform.translation = warp.offset_or_point.extend(0.0),
        false => player_transform.translation += warp.offset_or_point.extend(0.0),
    };

    camera.translation = player_transform.translation;

    let Vec3 { x, y, z } = player_transform.translation;
    info!(
        "Warping player to {:?} ({}, {}, {})",
        warp.target_map, x, y, z
    );

    let target_map_handle = topdown_maps[warp.target_map].clone_weak();

    commands.entity(*tiled_map).despawn();
    commands
        .spawn(TiledMapHandle(target_map_handle))
        .observe(setup_collider_bodies);
}

#[derive(Debug, Deref, DerefMut, Resource)]
struct LastPlayerLocation(Vec3);

impl Default for LastPlayerLocation {
    fn default() -> Self {
        const FIRST_SPAWN: Vec3 = vec3(832.0, 1024.0, 0.0);
        LastPlayerLocation(FIRST_SPAWN)
    }
}

fn spawn_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    last_location: Res<LastPlayerLocation>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let player_sprites: Handle<Image> = asset_server.load("sprites/bucko_bounce.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 8, 3, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    commands
        .spawn((
            OnTopDown,
            Player,
            InteractTarget(None),
            //
            Transform::from_translation(last_location.0),
            Visibility::default(),
            //
            Sprite::from_atlas_image(
                player_sprites,
                TextureAtlas {
                    layout: texture_atlas_layout,
                    index:  0,
                },
            ),
            HopState::Idle,
            Submerged::default(),
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
        .observe(update_player_animations);
}

fn update_player_z(
    mut player_transform: Single<&mut Transform, (With<Player>, Changed<Transform>)>,
    current_map: Res<CurrentMap>,
) {
    let offset_y = player_transform.translation.y - current_map.rect.min.y;
    player_transform.translation.z =
        -offset_y / current_map.rect.height() * Z_BETWEEN_LAYERS - Z_BETWEEN_LAYERS;
}

#[derive(Debug, Event)]
struct HopUpdate;

#[derive(Debug, Default, Component, Clone, Copy, PartialEq, Reflect)]
enum HopState {
    Ready,
    Charging,
    Airborne,
    Landing,
    Landed,
    #[default]
    Idle,
}

impl HopState {
    fn cycle_state(&mut self) {
        *self = match self {
            HopState::Ready => HopState::Charging,
            HopState::Charging => HopState::Airborne,
            HopState::Airborne => HopState::Landing,
            HopState::Landing => HopState::Landed,
            HopState::Landed => HopState::Idle,
            HopState::Idle => HopState::Idle,
        };
    }
}

fn player_hop(
    mut commands: Commands,
    user_input: Res<UserInput>,
    player: Single<(Entity, &mut HopState), With<Player>>,
) {
    let (entity, mut hop_state) = player.into_inner();

    if *hop_state == HopState::Idle && user_input.moving() {
        *hop_state = HopState::Ready;
        commands.trigger_targets(HopUpdate, entity);
    }
}

fn update_player_animations(
    trigger: Trigger<SpriteAnimationFinished>,
    mut commands: Commands,
    player: Single<(&mut HopState, Ref<Submerged>), With<Player>>,
) {
    let (mut hop_state, submerged) = player.into_inner();

    if submerged.is_changed() {
        *hop_state = HopState::Idle;
        commands.trigger_targets(SplashUpdate, trigger.target());
    }

    hop_state.cycle_state();
    commands.trigger_targets(HopUpdate, trigger.target());
}

fn update_player_hop(
    _trigger: Trigger<HopUpdate>,
    player: Single<(&HopState, &mut SpriteAnimation, &mut LinearVelocity), With<Player>>,
    user_input: Res<UserInput>,
    mut index_offset: Local<usize>,
) {
    const HOP_IMPULSE: f32 = 128.0;
    const SPRITES_PER_ROW: usize = 8;

    let (hop_state, mut animation, mut velocity) = player.into_inner();

    match hop_state {
        HopState::Ready => {
            // Map direction to spritesheet y-offset
            *index_offset = match user_input.last_valid_direction {
                Dir2::NORTH | Dir2::SOUTH => *index_offset,
                Dir2::EAST | Dir2::NORTH_EAST | Dir2::SOUTH_EAST => 0,
                Dir2::WEST | Dir2::NORTH_WEST | Dir2::SOUTH_WEST => SPRITES_PER_ROW,
                _ => unreachable!(),
            };
            *animation = SpriteAnimation::set_frame(*index_offset);
        }
        HopState::Charging => {
            *animation = SpriteAnimation::new(1 + *index_offset, 2 + *index_offset, 12);
        }
        HopState::Airborne => {
            *velocity = LinearVelocity(user_input.last_valid_direction * HOP_IMPULSE);
            *animation = SpriteAnimation::new(3 + *index_offset, 5 + *index_offset, 12);
        }
        HopState::Landing => {
            *animation = SpriteAnimation::new(6 + *index_offset, 7 + *index_offset, 12);
        }
        HopState::Landed => {
            *animation = SpriteAnimation::set_frame(*index_offset);
        }
        HopState::Idle => {}
    }
}

#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
struct WaterTile;

#[derive(Debug, Default, Deref, DerefMut, Component, PartialEq, Eq, Reflect)]
#[reflect(Default, Component)]
struct Submerged(bool);

#[derive(Debug, Event)]
struct SplashUpdate;

fn update_player_submerged(
    player: Single<(&Transform, &mut Submerged), (With<Player>, Changed<Transform>)>,
    current_map: Res<CurrentMap>,
    q_tiled_layers: Query<&TileStorage, With<TiledMapTileLayerForTileset>>,
    q_water_tiles: Query<&WaterTile, With<TiledMapTile>>,
) {
    let (transform, mut submerged) = player.into_inner();

    let Some(player_tile_pos) = get_tile_pos(transform.translation, &current_map.tilemap_size)
    else {
        debug_once!("Player outside map");
        return;
    };

    let Some(water_layer_entity) = current_map.water_layer else {
        debug_once!("Current map has no water layer");
        return;
    };

    let Ok(water_tile_storage) = q_tiled_layers.get(water_layer_entity) else {
        warn_once!("Failed to get tile storage for current map's water layer");
        return;
    };

    if let Some(current_tile) = water_tile_storage.get(&player_tile_pos) {
        submerged.set_if_neq(Submerged(q_water_tiles.contains(current_tile)));
    }
}

// fn player_splash(
//     player: Single<(&Submerged, &mut SpriteAnimation), (With<Player>, Changed<Submerged>)>,
// ) {

// }

fn player_swim(
    player: Single<(Ref<Submerged>, &mut SpriteAnimation, &mut LinearVelocity), With<Player>>,
    user_input: Res<UserInput>,
    mut index_offset: Local<usize>,
) {
    const SWIM_EAST_OFFSET: usize = 16;
    const SWIM_WEST_OFFSET: usize = 20;

    if *index_offset == 0 {
        *index_offset = SWIM_EAST_OFFSET;
    }

    let (submerged, mut sprite_animation, mut player_velocity) = player.into_inner();

    let new_offset = match user_input.last_valid_direction {
        Dir2::NORTH | Dir2::SOUTH => *index_offset,
        Dir2::NORTH_EAST | Dir2::SOUTH_EAST | Dir2::EAST => SWIM_EAST_OFFSET,
        Dir2::NORTH_WEST | Dir2::SOUTH_WEST | Dir2::WEST => SWIM_WEST_OFFSET,
        _ => unreachable!(),
    };

    if new_offset != *index_offset || submerged.is_changed() {
        *sprite_animation = SpriteAnimation::new(new_offset, new_offset + 3, 8).looping();
        *index_offset = new_offset
    }

    if user_input.moving() {
        *player_velocity = LinearVelocity(user_input.last_valid_direction * 32.0);
    }
}

fn player_submerged(player_submerged: Single<&Submerged, With<Player>>) -> bool {
    player_submerged.0
}

fn camera_system(
    mut camera_transform: Single<&mut Transform, With<WorldCamera>>,
    player: Single<&Transform, (With<Player>, Without<WorldCamera>)>,
    user_input: Res<UserInput>,
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

    let view_size = WINDOW_SIZE * camera_transform.scale.xy();

    if *target_direction != user_input.raw_vector
        && stopwatch.tick(time.delta()).elapsed_secs() > NEW_DIRECTION_DELAY
    {
        stopwatch.reset();
        *target_direction = user_input.raw_vector;
    }

    let camera_offset = (view_size * LOOKAHEAD * *target_direction).extend(0.0);
    let camera_target = player.translation + camera_offset;

    // Bounds camera from rendering the void outside the current map
    let camera_min = (current_map.rect.min + (view_size / 2.0) - HALF_TILE_SIZE).extend(f32::MIN);
    let camera_max = (current_map.rect.max - (view_size / 2.0) - HALF_TILE_SIZE).extend(f32::MAX);

    camera_transform.translation = camera_transform
        .translation
        .lerp(camera_target, SMOOTH)
        .clamp(camera_min, camera_max);
}

fn get_tile_pos(position: Vec3, tile_map_size: &TilemapSize) -> Option<TilePos> {
    static TILE_SIZE: f32 = 32.0;
    static OFFSET: f32 = 16.0;

    let tile_pos = TilePos::from(
        ((position.truncate() + OFFSET) / TILE_SIZE)
            .floor()
            .as_uvec2(),
    );

    tile_pos.within_map_bounds(tile_map_size).then_some(tile_pos)
}