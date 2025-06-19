use avian2d::PhysicsPlugins;
// use avian2d::prelude::PhysicsDebugPlugin;
use bevy::{
    // dev_tools::picking_debug::{DebugPickingMode, DebugPickingPlugin},
    prelude::*,
    render::{camera::CameraOutputMode, render_resource::*, view::RenderLayers},
    window::WindowResolution,
};
use bevy_ecs_tiled::{
    TiledMapPlugin, TiledMapPluginConfig,
    prelude::{TiledPhysicsAvianBackend, TiledPhysicsPlugin},
};
use bevy_persistent::prelude::*;
use bevy_rand::{plugin::EntropyPlugin, prelude::WyRand};
use bevy_text_animation::TextAnimatorPlugin;
use serde::{Deserialize, Serialize};
use vleue_kinetoscope::AnimatedImagePlugin;

use game::game_plugin;
use menu::menu_plugin;
use splash::splash_plugin;

mod game;
mod menu;
mod splash;

const WINDOW_WIDTH: u32 = 1280;
const WINDOW_HEIGHT: u32 = 720;

const RENDER_LAYER_WORLD: RenderLayers = RenderLayers::layer(0);
const RENDER_LAYER_2D: RenderLayers = RenderLayers::layer(1);

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
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32),
                    prevent_default_event_handling: false,
                    ..default()
                }),
                ..default()
            })
            .set(ImagePlugin::default_nearest()),
        MeshPickingPlugin,
        TextAnimatorPlugin,
        TiledMapPlugin(TiledMapPluginConfig {
            // Fixes crash on WASM
            // I don't think I need this...
            tiled_types_export_file: None,
        }),
        TiledPhysicsPlugin::<TiledPhysicsAvianBackend>::default(),
        PhysicsPlugins::default().with_length_unit(32.0),
        // PhysicsDebugPlugin::default(),
        EntropyPlugin::<WyRand>::default(),
        AnimatedImagePlugin,
        // DebugPickingPlugin,
    ))
    .add_plugins(bevy_inspector_egui::bevy_egui::EguiPlugin {
        enable_multipass_for_primary_context: true,
    })
    .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
    .add_plugins((menu_plugin, splash_plugin, game_plugin))
    .init_state::<AppState>()
    .add_systems(Startup, (setup, initialize_settings))
    // .insert_resource(DebugPickingMode::Normal)
    .insert_resource(avian2d::prelude::Gravity::ZERO)
    .insert_resource(MeshPickingSettings {
        require_markers:     true,
        ray_cast_visibility: RayCastVisibility::VisibleInView,
    })
    .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::Custom(Color::NONE),
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            },
            ..default()
        },
        // Msaa::Off,
        RENDER_LAYER_2D,
    ));

    // commands.spawn((
    //     vleue_kinetoscope::AnimatedImageController::play(asset_server.load("test.gif")),
    //     bevy::render::view::RenderLayers::layer(1),
    // ));
}

fn initialize_settings(mut commands: Commands) {
    let config_dir = dirs::config_dir()
        .map(|native_config_dir| native_config_dir.join("monkebucko"))
        .unwrap_or(std::path::Path::new("local").to_path_buf());

    commands.insert_resource(
        Persistent::<Settings>::builder()
            .name("settings")
            .format(StorageFormat::Toml)
            .path(config_dir.join("settings.toml"))
            .default(Settings::default())
            .build()
            .expect("failed to initialize settings"),
    )
}

// Generic system that takes a component as a parameter, and will despawn all entities with that component
fn despawn_screen<T: Component>(to_despawn: Query<Entity, With<T>>, mut commands: Commands) {
    for entity in &to_despawn {
        commands.entity(entity).despawn();
    }
}
