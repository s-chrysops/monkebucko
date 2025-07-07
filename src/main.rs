use avian2d::PhysicsPlugins;
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        camera::{CameraOutputMode, ImageRenderTarget, RenderTarget},
        render_resource::*,
        view::RenderLayers,
    },
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

use animation::sprite_animations_plugin;
use game::game_plugin;
use menu::menu_plugin;
use splash::splash_plugin;

use crate::{
    auto_scaling::ScalePlugin,
    viewport::{ViewportNode, viewport_node_plugin},
};

mod animation;
mod game;
mod menu;
mod splash;

mod auto_scaling;
mod viewport;

const WINDOW_WIDTH: u32 = 1280;
const WINDOW_HEIGHT: u32 = 720;

const ORDER_MAIN: isize = 2;
const ORDER_OVERLAY: isize = 1;
const ORDER_WORLD: isize = 0;

const RENDER_LAYER_MAIN: RenderLayers = RenderLayers::layer(ORDER_MAIN as usize);
const RENDER_LAYER_OVERLAY: RenderLayers = RenderLayers::layer(ORDER_OVERLAY as usize);
const RENDER_LAYER_WORLD: RenderLayers = RenderLayers::layer(ORDER_WORLD as usize);

#[derive(Debug, Component)]
struct CameraMain;

#[derive(Debug, Component)]
struct CameraOverlay;

#[derive(Debug, Component)]
struct CameraWorld;

#[derive(Debug, Component)]
struct ViewportNodeOverlay;

#[derive(Debug, Component)]
struct ViewportNodeWorld;

#[derive(Debug, Resource)]
struct RenderImageWorld(ImageRenderTarget);

#[derive(Debug, Resource, Serialize, Deserialize)]
struct Settings {
    up:       KeyCode,
    down:     KeyCode,
    left:     KeyCode,
    right:    KeyCode,
    jump:     KeyCode,
    swap:     KeyCode,
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
            swap:     KeyCode::KeyQ,
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
            // .set(PickingPlugin {
            //     is_window_picking_enabled: false,
            //     ..default()
            // })
            .set(ImagePlugin::default_nearest()),
        MeshPickingPlugin,
        TextAnimatorPlugin,
        ScalePlugin,
        TiledMapPlugin(TiledMapPluginConfig {
            // Fixes crash on WASM
            tiled_types_export_file: None,
            // ..default()
        }),
        TiledPhysicsPlugin::<TiledPhysicsAvianBackend>::default(),
        PhysicsPlugins::default().with_length_unit(32.0),
        EntropyPlugin::<WyRand>::default(),
        // avian2d::prelude::PhysicsDebugPlugin::default(),
        bevy::dev_tools::picking_debug::DebugPickingPlugin,
        // bevy_inspector_egui::bevy_egui::EguiPlugin {
        //     enable_multipass_for_primary_context: true,
        // },
        // bevy_inspector_egui::quick::WorldInspectorPlugin::new(),
    ))
    .add_plugins(viewport_node_plugin)
    .add_plugins((
        game_plugin,
        menu_plugin,
        splash_plugin,
        sprite_animations_plugin,
    ))
    .init_state::<AppState>()
    .add_systems(Startup, (setup_cameras, initialize_settings))
    .insert_resource(bevy::dev_tools::picking_debug::DebugPickingMode::Normal)
    .insert_resource(avian2d::prelude::Gravity::ZERO)
    .insert_resource(MeshPickingSettings {
        require_markers:     true,
        ray_cast_visibility: RayCastVisibility::VisibleInView,
    })
    .run();
}

fn setup_cameras(mut commands: Commands, asset_server: Res<AssetServer>) {
    let render_image_overlay: ImageRenderTarget = asset_server
        .add({
            let mut image = Image::new_uninit(
                default(),
                TextureDimension::D2,
                TextureFormat::Bgra8UnormSrgb,
                RenderAssetUsages::all(),
            );
            image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT;
            image
        })
        .into();

    let render_image_world: ImageRenderTarget = asset_server
        .add({
            let mut image = Image::new_uninit(
                default(),
                TextureDimension::D2,
                TextureFormat::Bgra8UnormSrgb,
                RenderAssetUsages::all(),
            );
            image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT;
            image
        })
        .into();
    commands.insert_resource(RenderImageWorld(render_image_world));

    use bevy::render::camera::ScalingMode;
    let overlay_camera = commands
        .spawn((
            CameraOverlay,
            Camera2d,
            Camera {
                order: ORDER_OVERLAY,
                target: RenderTarget::Image(render_image_overlay),
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
                    width:  WINDOW_WIDTH as f32,
                    height: WINDOW_HEIGHT as f32,
                },
                ..OrthographicProjection::default_3d()
            }),
            // Msaa::Off,
            RENDER_LAYER_OVERLAY,
        ))
        .id();

    commands.spawn((
        CameraMain,
        Camera2d,
        Camera {
            order: ORDER_MAIN,
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
                width:  WINDOW_WIDTH as f32,
                height: WINDOW_HEIGHT as f32,
            },
            ..OrthographicProjection::default_3d()
        }),
        RENDER_LAYER_MAIN,
    ));

    commands.spawn((
        ViewportNodeOverlay,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        ViewportNode::new(overlay_camera),
        ZIndex(ORDER_OVERLAY as i32),
        RENDER_LAYER_MAIN,
    ));
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

use bevy::{platform::collections::HashMap, prelude::TypePath};
use nohash_hasher::{IsEnabled, NoHashHasher};
use std::hash::{BuildHasherDefault, Hasher};

pub type BuckoNoHashHashmap<K, V> = HashMap<K, V, BuildHasherDefault<BuckoNoHashHasher<K>>>;
pub type BuildBuckoNoHashHasher<T> = BuildHasherDefault<BuckoNoHashHasher<T>>;

#[derive(Debug, Clone, Copy, Default, TypePath)]
pub struct BuckoNoHashHasher<T>(NoHashHasher<T>);

impl<T: IsEnabled> Hasher for BuckoNoHashHasher<T> {
    fn write(&mut self, _: &[u8]) {
        panic!("Invalid use of BuckoNoHashHasher")
    }

    fn write_u8(&mut self, n: u8) {
        self.0.write_u8(n);
    }
    fn write_u16(&mut self, n: u16) {
        self.0.write_u16(n);
    }
    fn write_u32(&mut self, n: u32) {
        self.0.write_u32(n);
    }
    fn write_u64(&mut self, n: u64) {
        self.0.write_u64(n);
    }
    fn write_usize(&mut self, n: usize) {
        self.0.write_usize(n);
    }

    fn write_i8(&mut self, n: i8) {
        self.0.write_i8(n);
    }
    fn write_i16(&mut self, n: i16) {
        self.0.write_i16(n);
    }
    fn write_i32(&mut self, n: i32) {
        self.0.write_i32(n);
    }
    fn write_i64(&mut self, n: i64) {
        self.0.write_i64(n);
    }
    fn write_isize(&mut self, n: isize) {
        self.0.write_isize(n);
    }

    fn finish(&self) -> u64 {
        self.0.finish()
    }
}
