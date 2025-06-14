use bevy::{
    // dev_tools::picking_debug::{DebugPickingMode, DebugPickingPlugin},
    prelude::*,
    render::{camera::CameraOutputMode, render_resource::*, view::RenderLayers},
    window::WindowResolution,
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

#[derive(Debug, Component)]
struct Canvas;

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
        EntropyPlugin::<WyRand>::default(),
        MeshPickingPlugin,
        TextAnimatorPlugin,
        AnimatedImagePlugin,
        // DebugPickingPlugin,
    ))
    .add_plugins((menu_plugin, splash_plugin, game_plugin))
    .init_state::<AppState>()
    .add_systems(Startup, setup)
    // .insert_resource(DebugPickingMode::Normal)
    .run();
}

fn setup(
    mut commands: Commands,
    // mut images: ResMut<Assets<Image>>,
    // asset_server: Res<AssetServer>,
    mut mesh_picking_settings: ResMut<MeshPickingSettings>,
) {
    // let canvas_size = Extent3d {
    //     width: WINDOW_WIDTH,
    //     height: WINDOW_HEIGHT,
    //     ..default()
    // };

    // let mut canvas = Image {
    //     texture_descriptor: TextureDescriptor {
    //         label:           None,
    //         size:            canvas_size,
    //         dimension:       TextureDimension::D2,
    //         format:          TextureFormat::Bgra8UnormSrgb,
    //         mip_level_count: 1,
    //         sample_count:    1,
    //         usage:           TextureUsages::TEXTURE_BINDING
    //             | TextureUsages::COPY_DST
    //             | TextureUsages::RENDER_ATTACHMENT,
    //         view_formats:    &[],
    //     },
    //     ..default()
    // };

    // canvas.resize(canvas_size);
    // let image_handle = images.add(canvas);

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
        // Transform::default(),
    ));

    // commands.spawn((
    //     vleue_kinetoscope::AnimatedImageController::play(asset_server.load("test.gif")),
    //     bevy::render::view::RenderLayers::layer(1),
    // ));

    // commands.spawn((
    //     Canvas,
    //     Sprite::from_image(image_handle),
    //     RENDER_LAYER_2D,
    //     Pickable::IGNORE,
    // ));

    mesh_picking_settings.require_markers = true;

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
