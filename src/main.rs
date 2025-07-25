use avian2d::PhysicsPlugins;
use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*,
    render::{camera::CameraOutputMode, render_resource::*, view::RenderLayers},
    window::WindowResolution,
};
use bevy_ecs_tiled::prelude::*;
use bevy_persistent::prelude::*;
use bevy_rand::{plugin::EntropyPlugin, prelude::WyRand};
use bevy_text_animation::TextAnimatorPlugin;
use serde::{Deserialize, Serialize};

use animation::sprite_animations_plugin;
use audio::audio_plugin;
use game::game_plugin;
use menu::menu_plugin;
use progress::progress_plugin;
use splash::splash_plugin;

use crate::auto_scaling::ScalePlugin;

mod animation;
mod audio;
mod game;
mod menu;
mod progress;
mod splash;

mod auto_scaling;

const WINDOW_WIDTH: f32 = 1280.0;
const WINDOW_HEIGHT: f32 = 720.0;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    Splash,
    Menu,
    Game {
        paused:   bool,
        can_move: bool,
    },
}

fn main() {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
                    prevent_default_event_handling: false,
                    ..default()
                }),
                ..default()
            })
            // .set(PickingPlugin {
            //     is_window_picking_enabled: false,
            //     ..default()
            // })
            .set(ImagePlugin::default_nearest()),
    );

    #[cfg(feature = "debug-pickings")]
    app.add_plugins(bevy::dev_tools::picking_debug::DebugPickingPlugin)
        .insert_resource(bevy::dev_tools::picking_debug::DebugPickingMode::Normal);

    #[cfg(feature = "debug-physics")]
    app.add_plugins(avian2d::prelude::PhysicsDebugPlugin::default());

    #[cfg(feature = "inspector")]
    app.add_plugins((
        bevy_inspector_egui::bevy_egui::EguiPlugin {
            enable_multipass_for_primary_context: true,
        },
        bevy_inspector_egui::quick::WorldInspectorPlugin::new(),
    ));

    #[cfg(feature = "export-types")]
    app.add_plugins(TiledPlugin(TiledPluginConfig::default()));

    #[cfg(not(feature = "export-types"))]
    app.add_plugins(TiledPlugin(TiledPluginConfig {
        tiled_types_export_file: None,
    }));

    app.add_plugins((
        MeshPickingPlugin,
        TextAnimatorPlugin,
        ScalePlugin,
        TiledPhysicsPlugin::<TiledPhysicsAvianBackend>::default(),
        PhysicsPlugins::default().with_length_unit(32.0),
        EntropyPlugin::<WyRand>::default(),
    ))
    .add_plugins((
        audio_plugin,
        game_plugin,
        menu_plugin,
        splash_plugin,
        sprite_animations_plugin,
        progress_plugin,
    ))
    .init_asset::<Blob>()
    .init_asset_loader::<BlobAssetLoader>()
    .init_state::<AppState>()
    .add_systems(Startup, (setup_overlay_camera, initialize_settings))
    .init_resource::<StandardFont>()
    .insert_resource(avian2d::prelude::Gravity::ZERO)
    .insert_resource(MeshPickingSettings {
        require_markers:     true,
        ray_cast_visibility: RayCastVisibility::VisibleInView,
    })
    .run();
}

const RENDER_LAYER_WORLD: RenderLayers = RenderLayers::layer(0);
const RENDER_LAYER_OVERLAY: RenderLayers = RenderLayers::layer(1);
const RENDER_LAYER_SPECIAL: RenderLayers = RenderLayers::layer(2);

#[derive(Debug, Component)]
struct OverlayCamera;

fn setup_overlay_camera(mut commands: Commands) {
    use bevy::render::camera::ScalingMode;
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
        auto_scaling::AspectRatio(16.0 / 9.0),
        Projection::from(OrthographicProjection {
            near: -1000.0,
            scaling_mode: ScalingMode::Fixed {
                width:  WINDOW_WIDTH,
                height: WINDOW_HEIGHT,
            },
            ..OrthographicProjection::default_3d()
        }),
        // Msaa::Off,
        RENDER_LAYER_OVERLAY,
    ));
}

#[derive(Debug, Deref, Resource)]
struct StandardFont(Handle<Font>);

impl FromWorld for StandardFont {
    fn from_world(world: &mut World) -> Self {
        let font = world.resource::<AssetServer>().load("Silkscreen.ttf");
        StandardFont(font)
    }
}

#[derive(Debug, Resource, Serialize, Deserialize)]
struct Settings {
    up:       KeyCode,
    down:     KeyCode,
    left:     KeyCode,
    right:    KeyCode,
    jump:     KeyCode,
    swap:     KeyCode,
    interact: KeyCode,

    sound_vol: f32,
    music_vol: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            up:       KeyCode::KeyW,
            down:     KeyCode::KeyS,
            left:     KeyCode::KeyA,
            right:    KeyCode::KeyD,
            jump:     KeyCode::Space,
            swap:     KeyCode::KeyQ,
            interact: KeyCode::KeyE,

            sound_vol: 1.0,
            music_vol: 1.0,
        }
    }
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

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::TypePath,
};
use nohash_hasher::{IsEnabled, NoHashHasher};
use std::hash::{BuildHasherDefault, Hasher};

pub type EnumMap<K, V> = HashMap<K, V, BuildHasherDefault<BuckoNoHashHasher<K>>>;
pub type EnumSet<T> = HashSet<T, BuildHasherDefault<BuckoNoHashHasher<T>>>;
pub type BuildBuckoNoHashHasher<T> = BuildHasherDefault<BuckoNoHashHasher<T>>;

#[derive(Debug, Clone, Copy, Default, TypePath)]
pub struct BuckoNoHashHasher<T>(NoHashHasher<T>);

impl<T: IsEnabled> Hasher for BuckoNoHashHasher<T> {
    fn write(&mut self, _: &[u8]) {
        panic!("Invalid use of BuckoNoHashHasher")
    }

    fn write_usize(&mut self, n: usize) {
        self.0.write_usize(n);
    }

    fn finish(&self) -> u64 {
        self.0.finish()
    }
}
// #[derive(Debug, Deref, DerefMut, Clone, Default, Reflect)]
// pub struct EnumMap<K, V>(HashMap<K, V, nohash_hasher::BuildNoHashHasher<K>>);

use thiserror::Error;

#[derive(Asset, TypePath, Debug)]
struct Blob {
    bytes: Vec<u8>,
}

#[derive(Default)]
struct BlobAssetLoader;

/// Possible errors that can be produced by [`BlobAssetLoader`]
#[non_exhaustive]
#[derive(Debug, Error)]
enum BlobAssetLoaderError {
    /// An [IO](std::io) Error
    #[error("Could not load file: {0}")]
    Io(#[from] std::io::Error),
}

impl AssetLoader for BlobAssetLoader {
    type Asset = Blob;
    type Settings = ();
    type Error = BlobAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        info!("Loading Blob...");
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        Ok(Blob { bytes })
    }
}
