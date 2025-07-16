#![allow(clippy::type_complexity)]
use avian2d::prelude::*;
use bevy::{prelude::*, time::Stopwatch};
use bevy_ecs_tiled::prelude::*;
use bevy_ecs_tilemap::tiles::TileStorage;

use super::*;
use crate::{
    RENDER_LAYER_WORLD, WINDOW_HEIGHT, WINDOW_WIDTH,
    animation::*,
    despawn_screen,
    game::{
        effects::*,
        interactions::{dialogue::DialoguePreload, *},
    },
};

#[derive(Debug, Component)]
struct OnTopDown;

#[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[source(GameState = GameState::TopDown)]
enum TopDownState {
    #[default]
    Loading,
    Ready,
    Warping,
}

pub fn topdown_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::TopDown), (setup_camera, setup_player))
        .add_systems(OnEnter(TopDownState::Loading), setup_map)
        .add_systems(
            Update,
            wait_for_ready.run_if(in_state(TopDownState::Loading)),
        )
        .add_systems(OnEnter(TopDownState::Ready), fade_from_whatever)
        .add_systems(
            Update,
            (
                camera_system,
                update_near_interactables,
                update_player_submerged,
                update_player_z,
                player_hop.run_if(not(player_submerged)),
                player_swim.run_if(player_submerged),
                get_topdown_interactions
                    .pipe(play_interactions)
                    .run_if(in_state(InteractionState::None).and(just_pressed_interact)),
            )
                .run_if(in_state(MovementState::Enabled))
                .run_if(in_state(TopDownState::Ready)),
        )
        .add_systems(OnEnter(TopDownState::Warping), fade_to_black)
        .add_systems(
            Update,
            warp_player.run_if(in_state(TopDownState::Warping).and(on_event::<FadeIn>)),
        )
        .add_systems(
            OnExit(GameState::TopDown),
            (despawn_screen::<OnTopDown>, reset_current_map),
        )
        .add_observer(setup_current_map)
        .add_sub_state::<TopDownState>()
        .init_resource::<CurrentMap>()
        .init_resource::<PlayerSpawnLocation>()
        .init_resource::<TopdownMapHandles>()
        .register_type::<HopState>()
        .register_type::<Submerged>()
        .register_type::<WaterTile>()
        .register_type::<Warp>();
}

fn setup_camera(mut commands: Commands, last_player_location: Res<PlayerSpawnLocation>) {
    use crate::auto_scaling::AspectRatio;
    use bevy::render::camera::ScalingMode;

    commands.spawn((
        OnTopDown,
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
                    width:  WINDOW_WIDTH,
                    height: WINDOW_HEIGHT,
                },
                ..OrthographicProjection::default_3d()
            }
        }),
        RENDER_LAYER_WORLD,
    ));
}

#[derive(Debug, Deref, DerefMut, Resource)]
pub struct PlayerSpawnLocation(pub Vec3);

impl Default for PlayerSpawnLocation {
    fn default() -> Self {
        const FIRST_SPAWN: Vec3 = vec3(832.0, 1024.0, 0.0);
        PlayerSpawnLocation(FIRST_SPAWN)
    }
}

fn setup_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    last_location: Res<PlayerSpawnLocation>,
) {
    let player_sprites: Handle<Image> = asset_server.load("sprites/bucko_bounce.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 8, 3, None, None);
    let texture_atlas_layout = asset_server.add(layout);

    commands
        .spawn((
            OnTopDown,
            Player,
            InteractTarget::default(),
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

fn setup_map(
    mut commands: Commands,
    spawned_tiled_maps: Query<Entity, With<TiledMapMarker>>,
    topdown_maps: Res<TopdownMapHandles>,
    current_map: Res<CurrentMap>,
) {
    spawned_tiled_maps.iter().for_each(|entity| {
        commands.entity(entity).despawn();
    });
    commands.remove_resource::<Warp>();

    let current_tiled_map = topdown_maps[current_map.index].clone_weak();
    commands
        .spawn(TiledMapHandle(current_tiled_map))
        .insert(OnTopDown)
        .observe(setup_collider_bodies)
        .observe(setup_interactables);
}

#[derive(Debug, Default, Resource)]
struct CurrentMap {
    ready: bool,
    index: TopdownMapIndex,

    rect:         Rect,
    tilemap_size: TilemapSize,

    entity:      Option<Entity>,
    water_layer: Option<Entity>,
}

fn reset_current_map(mut current_map: ResMut<CurrentMap>) {
    *current_map = CurrentMap::default();
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

    map_storage.objects.iter().for_each(|(_tiled_id, entity)| {
        if let Ok(mut transform) = q_tiled_objects.get_mut(*entity) {
            // Objects higher up on the map will be given a greater negative z-offset
            let offset_y = transform.translation.y - tiled_map.rect.min.y;
            transform.translation.z -= offset_y / tiled_map.rect.height() * Z_BETWEEN_LAYERS;
        }
    });

    current_map.ready = true;

    current_map.rect = tiled_map.rect;
    current_map.tilemap_size = tiled_map.tilemap_size;

    current_map.entity = Some(map_entity);
    current_map.water_layer = q_tiled_layers.iter().find_map(|(entity, name)| {
        (name.as_str() == "TiledMapTileLayerForTileset(water, water)").then_some(entity)
    });
}

fn setup_interactables(
    trigger: Trigger<TiledObjectCreated>,
    // mut commands: Commands,
    mut q_interactables: Query<(Entity, &EntityInteraction), With<TiledMapObject>>,
    mut dialogue_preloader: ResMut<DialoguePreload>,
) {
    if let Ok((_entity, EntityInteraction::Dialogue(id))) = q_interactables.get_mut(trigger.entity)
    {
        dialogue_preloader.push(*id);
        // commands
        //     .entity(entity)
        //     .insert(PICKABLE)
        //     .observe(over_interactables)
        //     .observe(out_interactables);
    }
}

fn setup_collider_bodies(
    trigger: Trigger<TiledColliderCreated>,
    mut commands: Commands,
    q_tiled_objects: Query<Option<&Warp>, With<TiledMapObject>>,
    q_tiled_colliders: Query<&ChildOf, With<TiledColliderMarker>>,
) {
    if let Ok(ChildOf(parent)) = q_tiled_colliders.get(trigger.entity) {
        if let Ok(Some(_warp)) = q_tiled_objects.get(*parent) {
            commands
                .entity(trigger.entity)
                .insert((Sensor, CollisionEventsEnabled))
                .observe(trigger_warp);
        }
    }

    commands.entity(trigger.entity).insert(RigidBody::Static);
}

fn wait_for_ready(
    current_map: Res<CurrentMap>,
    mut topdown_state: ResMut<NextState<TopDownState>>,
    mut movement_state: ResMut<NextState<MovementState>>,
) {
    if current_map.ready {
        topdown_state.set(TopDownState::Ready);
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

#[derive(Debug, Clone, Copy, Component, Resource, Reflect, Default)]
#[reflect(Component, Default)]
struct Warp {
    target_map:      TopdownMapIndex,
    point_mode:      bool,
    offset_or_point: Vec2,
}

fn trigger_warp(
    trigger: Trigger<OnCollisionStart>,
    player: Single<Entity, With<Player>>,
    q_tiled_colliders: Query<&ChildOf, With<TiledColliderMarker>>,
    q_tiled_objects: Query<&Warp, With<TiledMapObject>>,
    mut commands: Commands,
) {
    if trigger.collider != *player {
        // Something else it trying to warp
        return;
    }

    if let Ok(ChildOf(parent)) = q_tiled_colliders.get(trigger.target()) {
        if let Ok(warp) = q_tiled_objects.get(*parent) {
            commands.insert_resource(*warp);
            commands.set_state(TopDownState::Warping);
        }
    }
}

fn warp_player(
    warp: Res<Warp>,
    mut current_map: ResMut<CurrentMap>,
    mut player_transform: Single<&mut Transform, (With<Player>, Without<WorldCamera>)>,
    mut camera_transform: Single<&mut Transform, With<WorldCamera>>,
    mut topdown_state: ResMut<NextState<TopDownState>>,
) {
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

    camera_transform.translation = player_transform.translation;

    let Vec3 { x, y, z } = player_transform.translation;
    info!(
        "Warping player to {:?} ({}, {}, {})",
        warp.target_map, x, y, z
    );

    topdown_state.set(TopDownState::Loading);
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

fn update_near_interactables(
    q_interactables: Query<(Entity, &Transform), (With<EntityInteraction>, Without<Player>)>,
    player: Single<(&mut InteractTarget, &Transform), (With<Player>, Changed<Transform>)>,
) {
    const INTERACTION_RANGE: u32 = 32; // u32 to get a consistent value after squaring
    const RANGE_SQUARED: f32 = INTERACTION_RANGE.pow(2) as f32;

    let (mut interaction_target, player_transform) = player.into_inner();
    let player_position = player_transform.translation.truncate();

    let new_interaction_target =
        q_interactables
            .iter()
            .find_map(|(entity, interactable_transform)| {
                let interactable_position = interactable_transform.translation.truncate();
                let entity_in_range =
                    interactable_position.distance_squared(player_position) < RANGE_SQUARED;

                entity_in_range.then_some(entity)
            });

    match new_interaction_target {
        Some(entity) => interaction_target.set(entity),
        None => interaction_target.clear(),
    }
}

fn _over_interactables(
    over: Trigger<Pointer<Over>>,
    q_interactables: Query<Entity, With<EntityInteraction>>,
    mut interaction_target: Single<&mut InteractTarget, With<Player>>,
) {
    info!("OVER");
    if let Ok(target_entity) = q_interactables.get(over.target()) {
        interaction_target.set(target_entity);
    }
}

fn _out_interactables(
    out: Trigger<Pointer<Out>>,
    q_interactables: Query<Entity, With<EntityInteraction>>,
    mut interaction_target: Single<&mut InteractTarget, With<Player>>,
) {
    if let Ok(_target_entity) = q_interactables.get(out.target()) {
        interaction_target.clear();
    }
}

fn get_topdown_interactions(
    player: Single<(&InteractTarget, &Transform), With<Player>>,
    q_interactables: Query<(&mut EntityInteraction, &Transform)>,
) -> Option<EntityInteraction> {
    // const INTERACTION_RANGE: f32 = 32.0;

    let (interaction_target, _player_transform) = player.into_inner();
    let target_entity = interaction_target.as_ref()?;

    let (mut entity_interaction, _target_transform) =
        q_interactables.get_inner(*target_entity).ok()?;

    // let player_in_range = player_transform
    //     .translation
    //     .truncate()
    //     .distance(target_transform.translation.truncate())
    //     < INTERACTION_RANGE;

    // player_in_range.then_some(entity_interaction.clone())

    let entity_interaction = match entity_interaction.as_mut() {
        EntityInteraction::Dialogue(dialogue_id) => {
            EntityInteraction::Dialogue(std::mem::take(dialogue_id))
        }
        _ => entity_interaction.clone(),
    };

    Some(entity_interaction)
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
    const WINDOW_SIZE: Vec2 = Vec2::new(WINDOW_WIDTH, WINDOW_HEIGHT);
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

    tile_pos
        .within_map_bounds(tile_map_size)
        .then_some(tile_pos)
}
