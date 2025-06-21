#![allow(clippy::type_complexity)]
use std::time::Duration;

use avian2d::prelude::*;
use bevy::{prelude::*, time::Stopwatch};
use bevy_ecs_tiled::prelude::*;

use super::*;
use crate::{RENDER_LAYER_WORLD, WINDOW_HEIGHT, WINDOW_WIDTH};

#[derive(Debug, Component)]
struct OnTopDown;

#[derive(Debug, Event, Deref, PartialEq)]
struct SpriteAnimationFinished(Entity);

#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
struct SpriteAnimation {
    first_index: usize,
    last_index:  usize,
    frame_timer: Timer,
    looping:     bool,
}

impl SpriteAnimation {
    fn new(first: usize, last: usize, fps: u8) -> Self {
        SpriteAnimation {
            first_index: first,
            last_index:  last,
            frame_timer: Self::timer_from_fps(fps),
            looping:     false,
        }
    }

    fn _looping(mut self) -> Self {
        self.looping = true;
        self
    }

    // a little hack to change sprite without the Sprite component
    fn set_frame(index: usize) -> Self {
        SpriteAnimation {
            first_index: index,
            last_index:  index,
            frame_timer: Self::timer_from_fps(240),
            looping:     true,
        }
    }

    fn timer_from_fps(fps: u8) -> Timer {
        Timer::new(Duration::from_secs_f32(1.0 / (fps as f32)), TimerMode::Once)
    }
}

fn play_animations(
    mut commands: Commands,
    mut e_writer: EventWriter<SpriteAnimationFinished>,
    mut query: Query<(Entity, &mut SpriteAnimation, &mut Sprite)>,
    time: Res<Time>,
) {
    query
        .iter_mut()
        .for_each(|(entity, mut animation, mut sprite)| {
            animation.frame_timer.tick(time.delta());
            if animation.frame_timer.just_finished() {
                let atlas = sprite
                    .texture_atlas
                    .as_mut()
                    .expect("Animated Sprite with no Texture Atlas");

                if animation.first_index == animation.last_index {
                    atlas.index = animation.first_index;
                    e_writer.write(SpriteAnimationFinished(entity));
                    commands.trigger_targets(SpriteAnimationFinished(entity), entity);
                } else if atlas.index < animation.last_index {
                    atlas.index += 1;
                    animation.frame_timer.reset();
                } else if animation.looping {
                    atlas.index = animation.first_index;
                    animation.frame_timer.reset();
                } else {
                    e_writer.write(SpriteAnimationFinished(entity));
                    commands.trigger_targets(SpriteAnimationFinished(entity), entity);
                }
            }
        });
}

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
            (camera_system, play_animations, update_player_z).run_if(in_state(GameState::TopDown)),
        )
        .add_observer(set_tiled_object_z)
        .register_type::<MoveInput>()
        .register_type::<SpriteAnimation>()
        .init_resource::<MapRect>()
        .add_event::<SpriteAnimationFinished>();
}

fn topdown_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
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

    let buckoville: Handle<TiledMap> = asset_server.load("maps/buckoville.tmx");

    commands.spawn(TiledMapHandle(buckoville)).observe(
        |trigger: Trigger<TiledColliderCreated>, mut commands: Commands| {
            commands.entity(trigger.entity).insert(RigidBody::Static);
        },
    );
}

#[derive(Debug, Default, Deref, DerefMut, Resource)]
struct MapRect(Rect);

const Z_BETWEEN_LAYERS: f32 = 100.0;

fn set_tiled_object_z(
    trigger: Trigger<TiledMapCreated>,
    q_tiled_maps: Query<&mut TiledMapStorage, With<TiledMapMarker>>,
    mut q_tiled_objects: Query<&mut Transform, With<TiledMapObject>>,
    a_tiled_maps: Res<Assets<TiledMap>>,
    mut map_rect_cached: ResMut<MapRect>,
) {
    let Some(tiled_map) = trigger.event().get_map_asset(&a_tiled_maps) else {
        return;
    };
    let tilemap_size = tiled_map.tilemap_size;
    info!("Map Size = x: {}, y: {}", tilemap_size.x, tilemap_size.y);
    let map_rect = tiled_map.rect;
    map_rect_cached.0 = map_rect;

    let Ok(map_storage) = q_tiled_maps.get(trigger.entity) else {
        return;
    };

    // Objects higher up on the map will be given a greater negative z-offset
    map_storage.objects.iter().for_each(|(_tiled_id, entity)| {
        if let Ok(mut transform) = q_tiled_objects.get_mut(*entity) {
            let offset_y = transform.translation.y - map_rect.min.y;
            transform.translation.z -= offset_y / map_rect.height() * Z_BETWEEN_LAYERS;
        }
    });
}

fn update_player_z(
    player: Single<&mut Transform, (With<Player>, Changed<Transform>)>,
    map_rect: Res<MapRect>,
) {
    if map_rect.0 == Rect::default() {
        return;
    }
    let mut transform = player.into_inner();
    let offset_y = transform.translation.y - map_rect.min.y;
    transform.translation.z = -offset_y / map_rect.height() * Z_BETWEEN_LAYERS - Z_BETWEEN_LAYERS;
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
            MoveInput {
                input_vector:   Vec2::ZERO,
                last_direction: Dir2::EAST,
            },
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

// #[derive(Debug)]
// struct DelayTimer(Timer);

// impl Default for DelayTimer {
//     fn default() -> Self {
//         const MOVEMENT_DELAY: f32 = 0.0001;
//         Self(Timer::from_seconds(MOVEMENT_DELAY, TimerMode::Once))
//     }
// }

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
    map_rect: Res<MapRect>,
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
    let camera_min = (map_rect.min + (view_size / 2.0) - HALF_TILE_SIZE).extend(f32::MIN);
    let camera_max = (map_rect.max - (view_size / 2.0) - HALF_TILE_SIZE).extend(f32::MAX);

    camera_transform.translation = camera_transform
        .translation
        .lerp(camera_target, SMOOTH)
        .clamp(camera_min, camera_max);
}
