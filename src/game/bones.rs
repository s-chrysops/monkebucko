use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::*;

use crate::{RENDER_LAYER_WORLD, WINDOW_HEIGHT, WINDOW_WIDTH, despawn_screen};

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
        (setup_camera, setup_map, setup_player),
    )
    .add_systems(
        Update,
        wait_till_loaded.run_if(in_state(BonesState::Loading)),
    )
    .add_systems(
        Update,
        (
            progress,
            move_player.run_if(in_state(MovementState::Enabled)),
        )
            .run_if(in_state(BonesState::Playing)),
    )
    .add_systems(Update, conclude_bones.run_if(in_state(BonesState::Ending)))
    .add_systems(OnExit(GameState::Bones), despawn_screen::<OnBones>)
    // .add_systems(
    //     Update,
    //     (|state: Res<State<BonesState>>| info!("{:?}", **state))
    //         .run_if(state_changed::<BonesState>),
    // )
    .add_sub_state::<BonesState>()
    .init_resource::<BonesAssetTracker>()
    .init_resource::<BonesTimer>();
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

fn setup_player(
    mut commands: Commands,
    mut asset_tracker: ResMut<BonesAssetTracker>,
    asset_server: Res<AssetServer>,
) {
    const PLAYER_START: Vec3 = vec3(384.0, 160.0, 1.0);

    let player_image = asset_server.load("bucko.png");
    asset_tracker.push(player_image.clone().untyped());

    commands.spawn((
        OnBones,
        Player,
        //
        Transform::from_translation(PLAYER_START),
        Sprite {
            image: player_image,
            custom_size: Some(Vec2::splat(32.0)),
            ..default()
        },
        //
        RigidBody::Dynamic,
        Collider::circle(14.0),
        LockedAxes::ROTATION_LOCKED,
        LinearVelocity(vec2(32.0, 0.0)),
        //
        Visibility::default(),
        RENDER_LAYER_WORLD,
    ));
}

fn setup_map(
    mut commands: Commands,
    mut asset_tracker: ResMut<BonesAssetTracker>,
    asset_server: Res<AssetServer>,
) {
    let bones_map_handle = asset_server.load("maps/bones.tmx");
    asset_tracker.push(bones_map_handle.clone().untyped());

    commands
        .spawn(TiledMapHandle(bones_map_handle))
        .insert(OnBones)
        .observe(
            |trigger: Trigger<TiledColliderCreated>, mut commands: Commands| {
                commands.entity(trigger.entity).insert(RigidBody::Static);
            },
        );

    commands.insert_resource(Gravity::default());
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

fn progress(
    mut camera: Single<&mut Transform, With<WorldCamera>>,
    mut bones_state: ResMut<NextState<BonesState>>,
    mut timer: ResMut<BonesTimer>,
    time: Res<Time>,
) {
    const CAMERA_START_X: f32 = WINDOW_WIDTH / 4.0;
    const CAMERA_SPEED: f32 = 32.0; // pixels per second

    let time_seconds = timer.tick(time.delta()).elapsed_secs();
    camera.translation.x = (CAMERA_SPEED * time_seconds) + CAMERA_START_X;

    if timer.just_finished() {
        bones_state.set(BonesState::Ending);
    }
}

fn move_player(mut player: Single<&mut LinearVelocity, With<Player>>, user_input: Res<UserInput>) {
    player.x = (user_input.raw_vector.x * 16.0) + 32.0
}

fn conclude_bones(mut game_state: ResMut<NextState<GameState>>) {
    game_state.set(GameState::TopDown);
}
