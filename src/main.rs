use bevy::{prelude::*, render::view::RenderLayers};
use bevy_persistent::prelude::*;
use serde::{Deserialize, Serialize};

use game::game_plugin;
use menu::menu_plugin;
use splash::splash_plugin;

mod game;
mod menu;
mod splash;

// const UI_RENDER_LAYER: usize = 1;

#[derive(Debug, Resource, Serialize, Deserialize)]
struct Settings {
    up:       KeyCode,
    down:     KeyCode,
    left:     KeyCode,
    right:    KeyCode,
    jump:     KeyCode,
    interact: KeyCode,

    sound_vol: u32,
    music_vol: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            up:       KeyCode::KeyW,
            down:     KeyCode::KeyS,
            left:     KeyCode::KeyA,
            right:    KeyCode::KeyD,
            jump:     KeyCode::Space,
            interact: KeyCode::KeyE,

            sound_vol: 10,
            music_vol: 10,
        }
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum AppState {
    #[default]
    Splash,
    Menu,
    Game,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins((menu_plugin, splash_plugin, game_plugin))
        .init_state::<AppState>()
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        // Camera {
        //     order: 1,
        //     clear_color: ClearColorConfig::None,
        //     ..default()
        // },
        // RenderLayers::layer(UI_RENDER_LAYER),
    ));

    use bevy_persistent::Storage;
    let name = "settings";
    let format = StorageFormat::Toml;
    let storage = Storage::LocalStorage {
        key: "settings".to_owned(),
    };
    let loaded = true;
    let default = Settings::default();
    let revertible = false;
    let revert_to_default_on_deserialization_errors = false;

    commands.insert_resource(
        Persistent::new(
            name,
            format,
            storage,
            loaded,
            default,
            revertible,
            revert_to_default_on_deserialization_errors,
        )
        .expect("failed to initialize settings"),
    );
}

// Generic system that takes a component as a parameter, and will despawn all entities with that component
fn despawn_screen<T: Component>(to_despawn: Query<Entity, With<T>>, mut commands: Commands) {
    for entity in &to_despawn {
        commands.entity(entity).despawn();
    }
}
