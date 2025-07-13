use bevy::{color::palettes::css::*, prelude::*};
use bevy_persistent::Persistent;
use bevy_text_animation::*;

use super::{AppState, Settings};
use effects::effects_plugin;
use interactions::interactions_plugin;

pub mod effects;
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
    app.add_plugins((effects_plugin, interactions_plugin))
        .add_plugins((bones_plugin, egg_plugin, topdown_plugin))
        .add_systems(OnEnter(AppState::Game), game_setup)
        .add_systems(PreUpdate, get_user_input)
        .init_resource::<UserInput>()
        .init_state::<GameState>()
        .init_state::<MovementState>()
        .register_type::<UserInput>();
}

fn game_setup(mut commands: Commands) {
    commands.set_state(GameState::TopDown);
}

#[derive(Debug, Reflect)]
enum KeyState {
    Press,
    Hold,
    Release,
    Off,
}

#[derive(Debug, Resource, Reflect)]
#[reflect(Resource)]
struct UserInput {
    raw_vector:           Vec2,
    last_valid_direction: Dir2,

    jump:     KeyState,
    swap:     KeyState,
    interact: KeyState,
}

impl Default for UserInput {
    fn default() -> Self {
        UserInput {
            raw_vector:           Vec2::ZERO,
            last_valid_direction: Dir2::EAST,
            jump:                 KeyState::Off,
            swap:                 KeyState::Off,
            interact:             KeyState::Off,
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

    let [jump_state, swap_state, interact_state] =
        [settings.jump, settings.swap, settings.interact].map(|key| {
            if key_input.just_pressed(key) {
                KeyState::Press
            } else if key_input.pressed(key) {
                KeyState::Hold
            } else if key_input.just_released(key) {
                KeyState::Release
            } else {
                KeyState::Off
            }
        });

    user_input.jump = jump_state;
    user_input.swap = swap_state;
    user_input.interact = interact_state;
}

fn just_pressed_jump(user_input: Res<UserInput>) -> bool {
    matches!(user_input.jump, KeyState::Press)
}

fn _just_pressed_swap(user_input: Res<UserInput>) -> bool {
    matches!(user_input.swap, KeyState::Press)
}

fn just_pressed_interact(user_input: Res<UserInput>) -> bool {
    matches!(user_input.interact, KeyState::Press)
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
