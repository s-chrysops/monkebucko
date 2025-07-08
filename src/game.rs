use bevy::{color::palettes::css::*, prelude::*};
use bevy_persistent::Persistent;
use bevy_text_animation::*;

use super::{AppState, Settings};
use interactions::interactions_plugin;

pub mod interactions;

use bones::bones_plugin;
use egg::egg_plugin;
use topdown::topdown_plugin;

mod bones;
mod egg;
mod topdown;

const PICKABLE: Pickable = Pickable {
    should_block_lower: true,
    is_hoverable:       true,
};

const _Z_BASE: f32 = 0.0;
const Z_SPRITES: f32 = 1.0;
const Z_EFFECTS: f32 = 2.0;

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum GameState {
    #[default]
    Loading,
    Egg,
    TopDown,
    Bones,
}

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum MovementState {
    #[default]
    Enabled,
    Disabled,
}

#[derive(Debug, Component)]
struct Player;

#[derive(Debug, Component)]
struct WorldCamera;

pub fn game_plugin(app: &mut App) {
    app.add_plugins(interactions_plugin)
        .add_plugins((bones_plugin, egg_plugin, topdown_plugin))
        .add_systems(OnEnter(AppState::Game), game_setup)
        .add_systems(PreUpdate, get_user_input)
        .add_systems(Update, update_fade)
        .init_resource::<UserInput>()
        .init_state::<GameState>()
        .init_state::<MovementState>();
}

fn game_setup(mut commands: Commands) {
    commands.set_state(GameState::Egg);
}

#[derive(Debug, Resource)]
struct UserInput {
    raw_vector:           Vec2,
    last_valid_direction: Dir2,
}

impl Default for UserInput {
    fn default() -> Self {
        UserInput {
            raw_vector:           Vec2::ZERO,
            last_valid_direction: Dir2::EAST,
        }
    }
}

impl UserInput {
    pub fn moving(&self) -> bool {
        self.raw_vector != Vec2::ZERO
    }
}

fn get_user_input(
    key_input: Res<ButtonInput<KeyCode>>,
    settings: Res<Persistent<Settings>>,
    mut user_input: ResMut<UserInput>,
) {
    let mut raw_vector = Vec2::ZERO;
    if key_input.pressed(settings.up) {
        raw_vector += Vec2::Y;
    }
    if key_input.pressed(settings.down) {
        raw_vector -= Vec2::Y;
    }
    if key_input.pressed(settings.right) {
        raw_vector += Vec2::X;
    }
    if key_input.pressed(settings.left) {
        raw_vector -= Vec2::X;
    }
    user_input.raw_vector = raw_vector;

    if let Ok(new_direction) = Dir2::new(raw_vector) {
        user_input.last_valid_direction = new_direction;
    }
}

#[derive(Debug, Component)]
struct Fade;

#[derive(Clone, Component, Debug, Deref, DerefMut, Reflect)]
#[reflect(Component)]
struct Opacity(f32);

#[allow(clippy::type_complexity)]
fn update_fade(mut q_fade: Query<(&mut Sprite, &Opacity), (With<Fade>, Changed<Opacity>)>) {
    q_fade.iter_mut().for_each(|(mut sprite, Opacity(alpha))| {
        sprite.color = sprite.color.with_alpha(*alpha);
    });
}

fn pressed_interact_key(
    key_input: Res<ButtonInput<KeyCode>>,
    settings: Res<Persistent<Settings>>,
) -> bool {
    key_input.just_pressed(settings.interact)
}

fn pressed_advance_key(
    key_input: Res<ButtonInput<KeyCode>>,
    settings: Res<Persistent<Settings>>,
) -> bool {
    key_input.any_just_pressed([
        settings.jump,
        settings.interact,
        KeyCode::Escape,
        KeyCode::Enter,
        KeyCode::NumpadEnter,
    ])
}
