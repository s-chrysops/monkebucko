use bevy::{
    color::palettes::css::*,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use bevy_persistent::Persistent;
use bevy_rand::prelude::*;
use bevy_text_animation::*;
use rand_core::RngCore;

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
pub mod topdown;

const PICKABLE: Pickable = Pickable {
    should_block_lower: true,
    is_hoverable:       true,
};

const _Z_BASE: f32 = 0.0;
const Z_SPRITES: f32 = 1.0;
const Z_EFFECTS: f32 = 2.0;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InGame;

impl ComputedStates for InGame {
    type SourceStates = AppState;
    fn compute(sources: Self::SourceStates) -> Option<Self> {
        match sources {
            AppState::Game { .. } => Some(InGame),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates)]
#[source(InGame = InGame)]
enum GameState {
    #[default]
    Egg,
    TopDown,
    Bones,
}

#[derive(Debug, Component)]
struct Player;

#[derive(Debug, Component)]
struct WorldCamera;

#[derive(Debug, Component)]
struct SpecialCamera;

pub fn game_plugin(app: &mut App) {
    app.add_plugins((effects_plugin, interactions_plugin))
        .add_plugins((bones_plugin, egg_plugin, topdown_plugin))
        .add_systems(OnEnter(InGame), game_setup)
        .add_systems(PreUpdate, get_user_input)
        // .add_systems(
        //     Update,
        //     (|state: Res<State<AppState>>| info!("{:?}", state.get()))
        //         .run_if(state_changed::<AppState>),
        // )
        .add_computed_state::<InGame>()
        .add_computed_state::<MovementEnabled>()
        .add_sub_state::<GameState>()
        .init_resource::<AssetTracker>()
        .init_resource::<UserInput>()
        .register_type::<UserInput>();
}

fn game_setup(mut _commands: Commands) {
    // commands.set_state(GameState::Egg);
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct MovementEnabled;

impl ComputedStates for MovementEnabled {
    type SourceStates = AppState;
    fn compute(sources: Self::SourceStates) -> Option<Self> {
        match sources {
            AppState::Game {
                paused: false,
                can_move: true,
            } => Some(MovementEnabled),
            _ => None,
        }
    }
}

fn enable_movement(
    current_app_state: Res<State<AppState>>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    if let AppState::Game { paused, .. } = current_app_state.get() {
        next_app_state.set(AppState::Game {
            paused:   *paused,
            can_move: true,
        });
    }
}

fn disable_movement(
    current_app_state: Res<State<AppState>>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    if let AppState::Game { paused, .. } = current_app_state.get() {
        next_app_state.set(AppState::Game {
            paused:   *paused,
            can_move: false,
        });
    }
}

#[derive(Debug, Default, Deref, DerefMut, Resource)]
struct AssetTracker {
    #[deref]
    assets: Vec<UntypedHandle>,
    count:  u8,
}

impl AssetTracker {
    const CONFIRMATION_FRAMES: u8 = 4;
    fn is_ready(&mut self, asset_server: Res<'_, AssetServer>) -> bool {
        if !self.is_empty() {
            self.count = 0;
            self.retain(|asset| {
                // remove loaded assets from tracker
                asset_server
                    .get_recursive_dependency_load_state(asset)
                    .is_none_or(|state| !state.is_loaded())
            });
            false
        } else {
            self.count += 1;
            if self.count == Self::CONFIRMATION_FRAMES {
                self.count = 0;
                true
            } else {
                false
            }
        }
    }

    // fn add<A: Asset>(&mut self, handle: &Handle<A>){
    //     self.push(handle.clone_weak().untyped());
    // }
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

fn just_pressed_escape(key_input: Res<ButtonInput<KeyCode>>) -> bool {
    key_input.just_pressed(KeyCode::Escape)
}

fn just_pressed_jump(user_input: Res<UserInput>) -> bool {
    matches!(user_input.jump, KeyState::Press)
}

fn just_pressed_swap(user_input: Res<UserInput>) -> bool {
    matches!(user_input.swap, KeyState::Press)
}

fn just_pressed_interact(user_input: Res<UserInput>) -> bool {
    matches!(user_input.interact, KeyState::Press)
}

fn _pressing_jump(user_input: Res<UserInput>) -> bool {
    matches!(user_input.jump, KeyState::Press | KeyState::Hold)
}

fn _pressing_swap(user_input: Res<UserInput>) -> bool {
    matches!(user_input.swap, KeyState::Press | KeyState::Hold)
}

fn pressing_interact(user_input: Res<UserInput>) -> bool {
    matches!(user_input.interact, KeyState::Press | KeyState::Hold)
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

fn cursor_grab(mut primary_window: Single<&mut Window, With<PrimaryWindow>>) {
    primary_window.cursor_options.grab_mode = CursorGrabMode::Locked;
    primary_window.cursor_options.visible = false;
}

fn cursor_ungrab(mut primary_window: Single<&mut Window, With<PrimaryWindow>>) {
    primary_window.cursor_options.grab_mode = CursorGrabMode::None;
    primary_window.cursor_options.visible = true;
}

fn rng_percentage(rng: &mut Entropy<WyRand>, percent: f32) -> bool {
    rng.next_u32() < ((u32::MAX as f32 + 1.0) * percent.clamp(0.0, 1.0)) as u32
}

// Returns an f32 between two from a randomly generated u32
// From quad_rand <3
fn random_range(rng: &mut Entropy<WyRand>, low: f32, high: f32) -> f32 {
    let r = rng.next_u32() as f64 / (u32::MAX as f64 + 1.0);
    let r = low as f64 + (high as f64 - low as f64) * r;
    r as f32
}
