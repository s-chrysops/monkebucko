#![allow(clippy::type_complexity)]
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::*;

use super::*;
use crate::RENDER_LAYER_WORLD;

#[derive(Debug, Component)]
struct OnTopDown;

pub fn topdown_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::TopDown), topdown_setup)
        .add_systems(
            Update,
            move_player
                .run_if(in_state(GameState::TopDown))
                .run_if(in_state(MovementState::Enabled)),
        )
        .add_systems(Update, camera_follow.run_if(in_state(GameState::TopDown)));
}

fn topdown_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        WorldCamera,
        Camera2d,
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        RENDER_LAYER_WORLD,
    ));

    commands.spawn((
        OnTopDown,
        Player,
        InteractTarget(None),
        Transform::default(),
        Visibility::default(),
        Sprite::from_image(asset_server.load("bucko.png")),
        RENDER_LAYER_WORLD,
    ));

    let buckoville: Handle<TiledMap> = asset_server.load("maps/buckoville.tmx");

    commands.spawn(TiledMapHandle(buckoville));
}

const PLAYER_STEP: f32 = 6.0;

fn move_player(
    settings: Res<Persistent<Settings>>,
    key_input: Res<ButtonInput<KeyCode>>,
    player: Single<&mut Transform, With<Player>>,
) {
    let mut player_transform = player.into_inner();
    let mut next_position = player_transform.translation;
    let up = PLAYER_STEP * Vec3::Y;
    let right = PLAYER_STEP * Vec3::X;

    if key_input.pressed(settings.up) {
        next_position += up;
    }
    if key_input.pressed(settings.down) {
        next_position -= up;
    }
    if key_input.pressed(settings.left) {
        next_position -= right;
    }
    if key_input.pressed(settings.right) {
        next_position += right;
    }
    if key_input.pressed(settings.jump) {
        info!("JUMP!");
    }

    player_transform.translation = next_position;
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
