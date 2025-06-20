#![allow(clippy::type_complexity)]
use std::time::Duration;

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::*;

use super::*;
use crate::RENDER_LAYER_WORLD;

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
            (camera_follow, play_animations, update_player_z).run_if(in_state(GameState::TopDown)),
        )
        .add_observer(set_tiled_object_z)
        .register_type::<Hop>()
        .register_type::<SpriteAnimation>()
        .init_resource::<MapRect>()
        .add_event::<SpriteAnimationFinished>();
}

fn topdown_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    use crate::{WINDOW_WIDTH, WINDOW_HEIGHT};
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
        crate::auto_scaling::AspectRatio(16.0 / 9.0),
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
            Hop {
                state:          HopState::Idle,
                input_vector:   Vec2::ZERO,
                last_direction: Dir2::EAST,
            },
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

#[derive(Debug, Default, Clone, Copy, Reflect)]
enum HopState {
    Ready,
    Charging,
    Airborne,
    Landing,
    #[default]
    Idle,
}

#[derive(Debug, Component, Clone, Copy, Reflect)]
#[reflect(Component)]
struct Hop {
    state:          HopState,
    input_vector:   Vec2,
    last_direction: Dir2,
}

impl Hop {
    fn _update(&mut self, input: Vec2) {
        self.input_vector = input.normalize_or_zero();
        if let Ok(new_direction) = Dir2::new(input) {
            self.last_direction = new_direction;
            self.state = HopState::Ready;
        }
    }

    fn cycle_state(&mut self) {
        self.state = match self.state {
            HopState::Ready => HopState::Charging,
            HopState::Charging => HopState::Airborne,
            HopState::Airborne => HopState::Landing,
            HopState::Landing => HopState::Idle,
            HopState::Idle => HopState::Idle,
        }
    }
}

fn set_player_hop(
    mut commands: Commands,
    settings: Res<Persistent<Settings>>,
    key_input: Res<ButtonInput<KeyCode>>,
    player: Single<(Entity, &mut Hop), With<Player>>,
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

    let (entity, mut hop) = player.into_inner();
    hop.input_vector = input_vector.normalize_or_zero();
    if let Ok(new_direction) = Dir2::new(input_vector) {
        hop.last_direction = new_direction;
        hop.state = HopState::Ready;
        commands.trigger_targets(HopUpdate, entity);
    }
}

fn cycle_hop_animations(
    _trigger: Trigger<SpriteAnimationFinished>,
    mut commands: Commands,
    player: Single<(Entity, &mut Hop)>,
) {
    let (entity, mut hop) = player.into_inner();
    hop.cycle_state();
    commands.trigger_targets(HopUpdate, entity);
}

fn update_player_hop(
    _trigger: Trigger<HopUpdate>,
    mut commands: Commands,
    player: Single<(Entity, &mut Hop, &mut SpriteAnimation, &mut LinearVelocity)>,
) {
    const PLAYER_SPEED: f32 = 128.0;

    let (_entity, hop, mut animation, mut velocity) = player.into_inner();

    // if e_reader
    //     .read()
    //     .any(|ev| *ev == SpriteAnimationFinished(entity))
    // {
    //     hop.cycle_state();
    // }

    const SPRITES_PER_ROW: usize = 8;
    // Map direction to spritesheet y-offset
    // x.signum() will suffice whilst only right/left sprites are present
    let index_offset = SPRITES_PER_ROW
        * match hop.last_direction.x.signum() {
            // Facing right
            1.0 => 0,
            // Facing left
            -1.0 => 1,
            // Up or down, default to right
            _ => 0,
        };

    match hop.state {
        HopState::Ready => {
            commands.set_state(MovementState::Disabled);
            *animation = SpriteAnimation::set_frame(index_offset);
            // hop.cycle_state();
            // commands.trigger_targets(HopUpdate, entity);
        }
        HopState::Charging => {
            *animation = SpriteAnimation::new(1 + index_offset, 2 + index_offset, 12);
        }
        HopState::Airborne => {
            *velocity = LinearVelocity(hop.input_vector * PLAYER_SPEED);
            *animation = SpriteAnimation::new(3 + index_offset, 5 + index_offset, 12);
        }
        HopState::Landing => {
            *animation = SpriteAnimation::new(6 + index_offset, 7 + index_offset, 12);
        }
        HopState::Idle => {
            commands.set_state(MovementState::Enabled);
            *animation = SpriteAnimation::set_frame(index_offset);
            // if hop.input_vector != Vec2::ZERO {
            //     hop.cycle_state();
            //     commands.trigger_targets(HopUpdate, entity);
            // }
        }
    }
}

fn camera_follow(
    mut transforms: ParamSet<(
        Single<&mut Transform, With<WorldCamera>>,
        Single<&Transform, With<Player>>,
    )>,
) {
    let player_translation = transforms.p1().into_inner().translation;
    let mut camera_transform = transforms.p0().into_inner();

    let new_camera_translation = camera_transform.translation.lerp(player_translation, 0.1);
    camera_transform.translation = new_camera_translation;
}
