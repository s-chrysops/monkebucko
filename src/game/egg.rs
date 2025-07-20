use std::f32::consts::*;

use bevy::{animation::*, color::palettes::css::*, prelude::*};
use bevy_rand::prelude::*;
use rand_core::RngCore;

use crate::{
    RENDER_LAYER_OVERLAY, RENDER_LAYER_SPECIAL, RENDER_LAYER_WORLD,
    animation::{SpriteAnimation, SpriteAnimationFinished},
    auto_scaling::AspectRatio,
    despawn_screen,
    game::interactions::*,
};

use super::*;

#[derive(Debug, Component)]
struct OnEggScene;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, SubStates)]
#[source(GameState = GameState::Egg)]
enum EggState {
    #[default]
    Loading,
    Ready,
    Cracking,
}

#[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[source(EggState = EggState::Cracking)]
enum CrackingPhase {
    #[default]
    Easing,
    Punch,
    FastPunch,
    QuadPunch,
    Violence,
    Fading,
}

#[derive(Debug, Component, Deref, DerefMut)]
struct CameraSensitivity(Vec2);

impl Default for CameraSensitivity {
    fn default() -> Self {
        Self(vec2(0.003, 0.002))
    }
}

pub fn egg_plugin(app: &mut App) {
    app.add_systems(
        OnEnter(GameState::Egg),
        (
            setup_pointer,
            setup_player,
            setup_world,
            setup_stars,
            setup_cracking_elements,
            setup_crt,
            cursor_grab,
        ),
    )
    .add_systems(
        OnExit(GameState::Egg),
        (despawn_screen::<OnEggScene>, cursor_ungrab),
    )
    .add_systems(
        Update,
        update_pointer.run_if(on_event::<bevy::window::WindowResized>),
    )
    .add_systems(Update, wait_till_loaded.run_if(in_state(EggState::Loading)))
    .add_systems(
        OnEnter(EggState::Ready),
        (
            bind_render_target_to_panel,
            setup_interaction_observers,
            enable_movement,
        )
            .chain(),
    )
    .add_systems(
        Update,
        (
            update_stars_position,
            despawn_respawn_stars,
            (
                move_player,
                get_egg_interactions
                    .pipe(play_interactions)
                    .run_if(just_pressed_interact),
            )
                .run_if(in_state(MovementEnabled)),
        )
            .run_if(in_state(EggState::Ready)),
    )
    .add_systems(OnEnter(EggState::Cracking), setup_ease_and_play)
    .add_systems(
        Update,
        (
            advance_crack_phase.run_if(not(pressing_interact).and(just_pressed_swap)),
            exit_egg.run_if(just_pressed_jump),
            update_crack,
        )
            .run_if(in_state(EggState::Cracking)),
    )
    .add_systems(
        Update,
        (
            wait_for_ease.run_if(in_state(CrackingPhase::Easing)),
            punch.run_if(in_state(CrackingPhase::Punch)),
            fast_punch.run_if(in_state(CrackingPhase::FastPunch)),
            quad_punch.run_if(in_state(CrackingPhase::QuadPunch)),
            violence.run_if(in_state(CrackingPhase::Violence)),
            exit_wait_for_fade.run_if(in_state(CrackingPhase::Fading)),
        ),
    )
    .add_sub_state::<EggState>()
    .add_sub_state::<CrackingPhase>();
}

fn wait_till_loaded(
    asset_server: Res<AssetServer>,
    mut asset_tracker: ResMut<AssetTracker>,
    mut egg_state: ResMut<NextState<EggState>>,
) {
    if asset_tracker.is_ready(asset_server) {
        egg_state.set(EggState::Ready);
    }
}

// #[derive(Debug, Component, Deref, DerefMut)]
// struct Velocity(Vec3);

#[derive(Debug, Component)]
struct EggPointer;

fn setup_pointer(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    primary_window: Single<(Entity, &Window), With<PrimaryWindow>>,
) {
    use bevy::asset::uuid::Uuid;
    use bevy::picking::pointer::*;
    use bevy::render::camera::NormalizedRenderTarget;
    use bevy::window::WindowRef;

    let (window_entity, window) = primary_window.into_inner();
    let center = window.size() / 2.0;

    commands.spawn((
        OnEggScene,
        Name::new("EggCrosshair"),
        Sprite::from_image(asset_server.load("pointer.png")),
        Transform::default().with_scale(Vec3::splat(0.8)),
        RENDER_LAYER_OVERLAY,
    ));

    commands.spawn((
        OnEggScene,
        EggPointer,
        Name::new("EggPointer"),
        PointerId::Custom(Uuid::new_v4()),
        PointerLocation::new(Location {
            target:   NormalizedRenderTarget::Window(
                WindowRef::Primary.normalize(Some(window_entity)).unwrap(),
            ),
            position: center,
        }),
    ));
}

use bevy::picking::pointer::PointerLocation;

fn update_pointer(
    mut pointer: Single<&mut PointerLocation, With<EggPointer>>,
    primary_window: Single<&Window, With<PrimaryWindow>>,
) {
    if let Some(location) = pointer.location.as_mut() {
        location.position = primary_window.size() / 2.0;
    }
}

fn setup_player(mut commands: Commands) {
    use bevy::core_pipeline::{bloom::Bloom, tonemapping::Tonemapping};

    info!("Spawning player");

    commands.spawn((
        OnEggScene,
        Player,
        Name::new("Player"),
        InteractTarget::default(),
        CameraSensitivity::default(),
        Transform::from_xyz(0.0, 1.0, 0.0),
        Visibility::default(),
        children![(
            WorldCamera,
            MeshPickingCamera,
            Camera3d::default(),
            Camera {
                order: 0,
                hdr: true,
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            Tonemapping::TonyMcMapface,
            Bloom::NATURAL,
            Projection::from(PerspectiveProjection {
                fov: 70.0_f32.to_radians(),
                ..default()
            }),
            AspectRatio(16.0 / 9.0),
            RENDER_LAYER_WORLD,
        )],
    ));
}

#[derive(Debug, Component)]
struct Crack;

#[derive(Debug, Deref, DerefMut, Component)]
struct Health(u8);

const CRACK_HEALTH: u8 = 200;

#[derive(Debug, Deref, Component)]
struct CrackMaterials([Handle<StandardMaterial>; 6]);

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut asset_tracker: ResMut<AssetTracker>,
    asset_server: Res<AssetServer>,
) {
    info!("Spawning egg world");

    let scene_room = asset_server.load(GltfAssetLabel::Scene(0).from_asset("egg.glb"));
    asset_tracker.push(scene_room.clone_weak().untyped());
    commands.spawn((OnEggScene, SceneRoot(scene_room)));

    // Window Glass
    commands.spawn((
        OnEggScene,
        Mesh3d(meshes.add(Cuboid::new(2.02, 1.202, 0.03))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            specular_transmission: 0.95,
            diffuse_transmission: 1.0,
            thickness: 0.03,
            ior: 1.49,
            perceptual_roughness: 0.0,
            reflectance: 0.5,
            ..default()
        })),
        Transform::from_xyz(0.3, 1.4, 1.525),
        PICKABLE,
        EntityInteraction::Text(
            "Through eons of void, these photons birth from fusion, lay to rest in you."
                .to_string(),
        ),
    ));

    // Star light
    commands.spawn((
        OnEggScene,
        DirectionalLight {
            color: Color::from(LAVENDER),
            illuminance: 4.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 1.4, 4.0),
    ));

    const CRACK_PATHS: [&str; 6] = [
        "sprites/crack/crack1.png",
        "sprites/crack/crack2.png",
        "sprites/crack/crack3.png",
        "sprites/crack/crack4.png",
        "sprites/crack/crack5.png",
        "sprites/crack/crack6.png",
    ];

    let crack_materials = CrackMaterials(CRACK_PATHS.map(|path| {
        let image = asset_server.load(path);
        asset_tracker.push(image.clone_weak().untyped());
        materials.add(StandardMaterial {
            base_color_texture: Some(image),
            perceptual_roughness: 1.0,
            alpha_mode: AlphaMode::Mask(0.5),
            cull_mode: None,
            emissive: LinearRgba::rgb(150.0, 150.0, 150.0),
            ..default()
        })
    }));

    // Crack
    let crack_entity = commands.spawn_empty().id();
    commands.entity(crack_entity).insert((
        OnEggScene,
        Crack,
        Health(CRACK_HEALTH),
        Transform::from_xyz(1.49, 1.0, 0.5).with_rotation(Quat::from_rotation_y(-FRAC_PI_2)),
        Mesh3d(meshes.add(Rectangle::new(1.0, 1.0))),
        MeshMaterial3d(crack_materials[0].clone_weak()),
        crack_materials,
        PICKABLE,
        EntityInteraction::Special(crack_entity),
        SpecialInteraction::new(move |commands: &mut Commands, _entity: Entity| {
            commands.set_state(EggState::Cracking);
        }),
    ));

    commands.spawn((
        OnEggScene,
        PointLight {
            intensity: 8192.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 2.9, 0.0),
    ));
}

#[derive(Debug, Component)]
struct CrtSprite;

fn setup_crt(
    mut commands: Commands,
    mut asset_tracker: ResMut<AssetTracker>,
    asset_server: Res<AssetServer>,
) {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

    // This is the texture that will be rendered to.
    let mut image = Image::new_fill(
        Extent3d {
            width: 128,
            height: 96,
            ..default()
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );

    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    let image_handle = asset_server.add(image);

    use bevy::render::camera::{RenderTarget, ScalingMode};
    commands.spawn((
        OnEggScene,
        SpecialCamera,
        Camera2d,
        Camera {
            order: 2,
            target: RenderTarget::Image(image_handle.into()),
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Projection::from(OrthographicProjection {
            near: -1000.0,
            scaling_mode: ScalingMode::Fixed {
                width:  200.0,
                height: 150.0,
            },
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_rotation(Quat::from_rotation_z(PI)),
        RENDER_LAYER_SPECIAL,
    ));

    let ami_intro = asset_server.load("sprites/ami_intro.png");
    asset_tracker.push(ami_intro.clone_weak().untyped());
    let ami_layout = TextureAtlasLayout::from_grid(uvec2(128, 96), 10, 6, None, None);
    let ami_layout = asset_server.add(ami_layout);

    commands.spawn((
        OnEggScene,
        CrtSprite,
        Sprite::from_atlas_image(ami_intro, ami_layout.into()),
        SpriteAnimation::set_frame(59),
        Transform::default(),
        Visibility::default(),
        RENDER_LAYER_SPECIAL,
    ));
}

use bevy::gltf::GltfMaterialName;

fn bind_render_target_to_panel(
    q_gltf_materials: Query<(Entity, &GltfMaterialName, &MeshMaterial3d<StandardMaterial>)>,
    special_camera: Single<&Camera, With<SpecialCamera>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    if let Some((entity, material_handle)) = q_gltf_materials
        .iter()
        .find_map(|(entity, name, handle)| (name.0 == "CRT_Panel").then_some((entity, handle)))
    {
        if let Some(material) = materials.get_mut(&material_handle.0) {
            if let Some(render_image) = special_camera.target.as_image() {
                material.base_color_texture = Some(render_image.clone());
                info!("CRT Panel Texture set to Special Camera render target");
            }
        }
        commands
            .entity(entity)
            .insert((PICKABLE, EntityInteraction::Text("amogus".to_string())));
    }
}

fn setup_interaction_observers(
    q_interactables: Query<Entity, With<EntityInteraction>>,
    mut commands: Commands,
) {
    let mut observer_over = Observer::new(over_interactables);
    let mut observer_out = Observer::new(out_interactables);

    q_interactables.iter().for_each(|entity| {
        observer_over.watch_entity(entity);
        observer_out.watch_entity(entity);
    });

    commands.spawn(observer_over);
    commands.spawn(observer_out);
}

const ROOM_BOUNDARY: Vec3 = Vec3::splat(1.3);
const PLAYER_STEP: f32 = 0.04;

use bevy::input::mouse::AccumulatedMouseMotion;

fn move_player(
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    user_input: Res<UserInput>,
    player: Single<(&mut Transform, &CameraSensitivity), With<Player>>,
) {
    let (mut transform, camera_sensitivity) = player.into_inner();

    let mouse_delta = accumulated_mouse_motion.delta;
    let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);

    if mouse_delta != Vec2::ZERO {
        let delta_yaw = -mouse_delta.x * camera_sensitivity.x;
        let delta_pitch = -mouse_delta.y * camera_sensitivity.y;

        let yaw = yaw + delta_yaw;

        // If the pitch was ±¹⁄₂ π, the camera would look straight up or down.
        // When the user wants to move the camera back to the horizon, which way should the camera face?
        // The camera has no way of knowing what direction was "forward" before landing in that extreme position,
        // so the direction picked will for all intents and purposes be arbitrary.
        // Another issue is that for mathematical reasons, the yaw will effectively be flipped when the pitch is at the extremes.
        // To not run into these issues, we clamp the pitch to a safe range.
        const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;
        let pitch = (pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);

        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
    }

    if user_input.moving() {
        const VECTOR_MAP: Vec2 = vec2(0.5 * PLAYER_STEP, -PLAYER_STEP);

        let translation = Vec2::from_angle(-yaw)
            .rotate(user_input.last_valid_direction.as_vec2() * VECTOR_MAP)
            .extend(0.0)
            .xzy();

        let next_position = transform.translation + translation;

        transform.translation = next_position.clamp(-ROOM_BOUNDARY, ROOM_BOUNDARY);
        // transform.translation = next_position;
    }
}

#[derive(Debug, PartialEq)]
enum CrackingElementId {
    PunchLower,
    PunchUpper,
    Guns,
}

#[derive(Debug, Component)]
struct CrackingElementInfo {
    id:       CrackingElementId,
    parts:    Vec<Entity>,
    in_node:  AnimationNodeIndex,
    out_node: AnimationNodeIndex,
}

fn setup_cracking_elements(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Default animation duration: 1.0 second
    // const MEDIUM_DURATION: f32 = 0.5;
    const FAST_DURATION: f32 = 0.125;

    const CCW90_ROTATION: Quat = Quat::from_array([0.0, 0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2]);
    const CW90_ROTATION: Quat = Quat::from_array([0.0, 0.0, -FRAC_1_SQRT_2, FRAC_1_SQRT_2]);

    let punch_image: Handle<Image> = asset_server.load("sprites/punch.png");
    let punch_layout = TextureAtlasLayout::from_grid(UVec2::splat(256), 2, 1, None, None);
    let punch_layout_handle = asset_server.add(punch_layout);

    {
        let punch_lr_sprite = Sprite {
            image: punch_image.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: punch_layout_handle.clone(),
                index:  0,
            }),
            custom_size: Some(Vec2::splat(720.0)),
            ..default()
        };

        let punch_ll_sprite = Sprite {
            flip_x: true,
            ..punch_lr_sprite.clone()
        };

        let punch_lower_name = Name::new("Punch Lower");
        let punch_lower_id = AnimationTargetId::from_name(&punch_lower_name);

        let punch_lower_in = Vec3::new(0.0, -64.0, 0.0);
        let punch_lower_out = Vec3::new(0.0, -512.0, 0.0);

        let (punch_lower_graph, punch_lower_nodes) = AnimationGraph::from_clips([
            asset_server.add({
                let mut punch_lower_in_clip = AnimationClip::default();
                punch_lower_in_clip.add_curve_to_target(
                    punch_lower_id,
                    AnimatableCurve::new(
                        animated_field!(Transform::translation),
                        EasingCurve::new(punch_lower_out, punch_lower_in, EaseFunction::BackOut),
                    ),
                );
                punch_lower_in_clip
            }),
            asset_server.add({
                let mut punch_lower_out_clip = AnimationClip::default();
                punch_lower_out_clip.add_curve_to_target(
                    punch_lower_id,
                    AnimatableCurve::new(
                        animated_field!(Transform::translation),
                        EasingCurve::new(punch_lower_in, punch_lower_out, EaseFunction::Linear)
                            .reparametrize_linear(Interval::new(0.0, FAST_DURATION).unwrap())
                            .unwrap(),
                    ),
                );
                punch_lower_out_clip
            }),
        ]);

        let punch_lower_graph_handle = asset_server.add(punch_lower_graph);

        let punch_lower_parts = [
            commands
                .spawn((
                    punch_lr_sprite,
                    SpriteAnimation::set_frame(0),
                    Transform::from_xyz(256.0, 0.0, Z_SPRITES),
                    RENDER_LAYER_OVERLAY,
                ))
                .id(),
            commands
                .spawn((
                    punch_ll_sprite,
                    SpriteAnimation::set_frame(0),
                    Transform::from_xyz(-256.0, 0.0, Z_SPRITES),
                    RENDER_LAYER_OVERLAY,
                ))
                .id(),
        ];

        let punch_lower = commands
            .spawn((
                OnEggScene,
                punch_lower_name,
                CrackingElementInfo {
                    id:       CrackingElementId::PunchLower,
                    parts:    punch_lower_parts.to_vec(),
                    in_node:  punch_lower_nodes[0],
                    out_node: punch_lower_nodes[1],
                },
                Transform::from_translation(punch_lower_out),
                AnimationPlayer::default(),
                AnimationGraphHandle(punch_lower_graph_handle),
                Visibility::default(),
                RENDER_LAYER_OVERLAY,
            ))
            .id();
        commands
            .entity(punch_lower)
            .insert(AnimationTarget {
                id:     punch_lower_id,
                player: punch_lower,
            })
            .add_children(&punch_lower_parts);
    }

    {
        let punch_upper = commands
            .spawn((
                OnEggScene,
                Transform::default(),
                AnimationPlayer::default(),
                Visibility::default(),
                RENDER_LAYER_OVERLAY,
            ))
            .id();

        let punch_ur_sprite = Sprite {
            image: punch_image.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: punch_layout_handle.clone(),
                index:  0,
            }),
            custom_size: Some(Vec2::splat(720.0)),
            ..default()
        };

        let punch_ul_sprite = Sprite {
            flip_x: true,
            ..punch_ur_sprite.clone()
        };

        let punch_ur_name = Name::new("Punch Lower Right");
        let punch_ul_name = Name::new("Punch Lower Left");
        let punch_ur_id = AnimationTargetId::from_name(&punch_ur_name);
        let punch_ul_id = AnimationTargetId::from_name(&punch_ul_name);

        let punch_ur_in = Vec3::new(280.0, 128.0, Z_SPRITES + 0.1);
        let punch_ur_out = Vec3::new(636.0, 128.0, Z_SPRITES + 0.1);
        let punch_ul_in = Vec3::new(-280.0, 128.0, Z_SPRITES + 0.1);
        let punch_ul_out = Vec3::new(-636.0, 128.0, Z_SPRITES + 0.1);

        let (punch_upper_graph, punch_upper_nodes) = AnimationGraph::from_clips([
            asset_server.add({
                let mut punch_upper_in_clip = AnimationClip::default();
                punch_upper_in_clip.add_curve_to_target(
                    punch_ur_id,
                    AnimatableCurve::new(
                        animated_field!(Transform::translation),
                        EasingCurve::new(punch_ur_out, punch_ur_in, EaseFunction::QuadraticOut),
                    ),
                );
                punch_upper_in_clip.add_curve_to_target(
                    punch_ul_id,
                    AnimatableCurve::new(
                        animated_field!(Transform::translation),
                        EasingCurve::new(punch_ul_out, punch_ul_in, EaseFunction::QuadraticOut),
                    ),
                );
                punch_upper_in_clip
            }),
            asset_server.add({
                let mut punch_upper_out_clip = AnimationClip::default();
                punch_upper_out_clip.add_curve_to_target(
                    punch_ur_id,
                    AnimatableCurve::new(
                        animated_field!(Transform::translation),
                        EasingCurve::new(punch_ur_in, punch_ur_out, EaseFunction::Linear)
                            .reparametrize_linear(Interval::new(0.0, FAST_DURATION).unwrap())
                            .unwrap(),
                    ),
                );
                punch_upper_out_clip.add_curve_to_target(
                    punch_ul_id,
                    AnimatableCurve::new(
                        animated_field!(Transform::translation),
                        EasingCurve::new(punch_ul_in, punch_ul_out, EaseFunction::Linear)
                            .reparametrize_linear(Interval::new(0.0, FAST_DURATION).unwrap())
                            .unwrap(),
                    ),
                );
                punch_upper_out_clip
            }),
        ]);

        let punch_upper_graph_handle = asset_server.add(punch_upper_graph);

        let punch_upper_parts = [
            commands
                .spawn((
                    punch_ur_sprite,
                    SpriteAnimation::set_frame(0),
                    AnimationTarget {
                        id:     punch_ur_id,
                        player: punch_upper,
                    },
                    Transform::from_translation(punch_ur_out).with_rotation(CCW90_ROTATION),
                    RENDER_LAYER_OVERLAY,
                ))
                .id(),
            commands
                .spawn((
                    punch_ul_sprite,
                    SpriteAnimation::set_frame(0),
                    AnimationTarget {
                        id:     punch_ul_id,
                        player: punch_upper,
                    },
                    Transform::from_translation(punch_ul_out).with_rotation(CW90_ROTATION),
                    RENDER_LAYER_OVERLAY,
                ))
                .id(),
        ];

        commands
            .entity(punch_upper)
            .add_children(&punch_upper_parts)
            .insert((
                CrackingElementInfo {
                    id:       CrackingElementId::PunchUpper,
                    parts:    punch_upper_parts.to_vec(),
                    in_node:  punch_upper_nodes[0],
                    out_node: punch_upper_nodes[1],
                },
                AnimationGraphHandle(punch_upper_graph_handle),
            ));
    }

    {
        let guns = commands
            .spawn((
                OnEggScene,
                AnimationPlayer::default(),
                Transform::default(),
                Visibility::default(),
                RENDER_LAYER_OVERLAY,
            ))
            .id();

        struct PreElement {
            name: &'static str,

            path:        &'static str,
            layout:      TextureAtlasLayout,
            flip_x:      bool,
            custom_size: Option<Vec2>,

            rotation:        Quat,
            translation_in:  Vec3,
            translation_out: Vec3,
        }

        const PADDING: UVec2 = UVec2::splat(16);

        let pre_elements = [
            PreElement {
                name:            "machgun",
                path:            "sprites/machgun.png",
                layout:          TextureAtlasLayout::from_grid(
                    uvec2(256, 132),
                    4,
                    1,
                    Some(PADDING),
                    None,
                ),
                flip_x:          false,
                custom_size:     Some(vec2(512.0, 264.0)),
                rotation:        Quat::default(),
                translation_in:  vec3(-270.0, -228.0, Z_SPRITES + 0.2),
                translation_out: vec3(-270.0, -374.0, Z_SPRITES + 0.2),
            },
            PreElement {
                name:            "shotgun",
                path:            "sprites/shotgun.png",
                layout:          TextureAtlasLayout::from_grid(
                    uvec2(138, 156),
                    5,
                    3,
                    Some(PADDING),
                    None,
                ),
                flip_x:          false,
                custom_size:     Some(vec2(276.0, 312.0)),
                rotation:        Quat::default(),
                translation_in:  vec3(500.0, -206.0, Z_SPRITES + 0.2),
                translation_out: vec3(500.0, -414.0, Z_SPRITES + 0.2),
            },
            PreElement {
                name:            "pistol1",
                path:            "sprites/pistol1.png",
                layout:          TextureAtlasLayout::from_grid(
                    uvec2(152, 152),
                    4,
                    1,
                    Some(PADDING),
                    None,
                ),
                flip_x:          false,
                custom_size:     Some(vec2(304.0, 304.0)),
                rotation:        CCW90_ROTATION,
                translation_in:  vec3(488.0, 152.0, Z_SPRITES + 0.2),
                translation_out: vec3(684.0, 152.0, Z_SPRITES + 0.2),
            },
            PreElement {
                name:            "pistol2",
                path:            "sprites/pistol2.png",
                layout:          TextureAtlasLayout::from_grid(
                    uvec2(112, 132),
                    4,
                    1,
                    Some(PADDING),
                    None,
                ),
                flip_x:          true,
                custom_size:     Some(vec2(224.0, 264.0)),
                rotation:        CW90_ROTATION,
                translation_in:  vec3(-507.0, 96.0, Z_SPRITES + 0.2),
                translation_out: vec3(-706.0, 96.0, Z_SPRITES + 0.2),
            },
        ];

        let (elements, in_out_curves): (
            Vec<Entity>,
            Vec<(
                AnimationTargetId,
                AnimatableCurve<_, _>,
                AnimatableCurve<_, _>,
            )>,
        ) = pre_elements
            .into_iter()
            .map(|pre_element| {
                let PreElement {
                    name,
                    path,
                    layout,
                    flip_x,
                    custom_size,
                    rotation,
                    translation_in,
                    translation_out,
                } = pre_element;

                let name = Name::new(name);
                let target_id = AnimationTargetId::from_name(&name);

                let element_entity = commands
                    .spawn((
                        Sprite {
                            image: asset_server.load(path),
                            texture_atlas: Some(TextureAtlas {
                                layout: asset_server.add(layout),
                                index:  0,
                            }),
                            flip_x,
                            custom_size,
                            ..default()
                        },
                        SpriteAnimation::set_frame(0),
                        AnimationTarget {
                            id:     target_id,
                            player: guns,
                        },
                        Transform::from_translation(translation_out).with_rotation(rotation),
                        Visibility::default(),
                        RENDER_LAYER_OVERLAY,
                        name,
                    ))
                    .id();

                let in_out_curves = (
                    target_id,
                    AnimatableCurve::new(
                        animated_field!(Transform::translation),
                        EasingCurve::new(translation_out, translation_in, EaseFunction::Linear)
                            .reparametrize_linear(Interval::new(0.0, FAST_DURATION).unwrap())
                            .unwrap(),
                    ),
                    AnimatableCurve::new(
                        animated_field!(Transform::translation),
                        EasingCurve::new(translation_in, translation_out, EaseFunction::Linear)
                            .reparametrize_linear(Interval::new(0.0, FAST_DURATION).unwrap())
                            .unwrap(),
                    ),
                );

                (element_entity, in_out_curves)
            })
            .unzip();

        let guns_clips = in_out_curves.into_iter().fold(
            [AnimationClip::default(), AnimationClip::default()],
            |[mut clip_in, mut clip_out], (target_id, curve_in, curve_out)| {
                [
                    {
                        clip_in.add_curve_to_target(target_id, curve_in);
                        clip_in
                    },
                    {
                        clip_out.add_curve_to_target(target_id, curve_out);
                        clip_out
                    },
                ]
            },
        );

        let (guns_graph, guns_nodes) =
            AnimationGraph::from_clips(guns_clips.map(|clip| asset_server.add(clip)));
        let guns_graph_handle = asset_server.add(guns_graph);

        commands.entity(guns).add_children(&elements).insert((
            CrackingElementInfo {
                id:       CrackingElementId::Guns,
                parts:    elements,
                in_node:  guns_nodes[0],
                out_node: guns_nodes[1],
            },
            AnimationGraphHandle(guns_graph_handle),
        ));
    }
}

#[derive(Debug, Component)]
struct StarRoot;

#[derive(Debug, Component)]
// Star with parallax speed
struct Star {
    speed: f32,
}

const MAX_STAR_AMOUNT: usize = 300;
const BACK_STAR_AMOUNT: usize = 200;

const MIN_STAR_SPEED: f32 = 0.01;
const MAX_STAR_SPEED: f32 = 0.5;

const MIN_STAR_HEIGHT: f32 = -15.0;
const MAX_STAR_HEIGHT: f32 = 30.0;

const LUMINANCE_LEVELS: usize = 4;
const MIN_STAR_LUMINANCE: f32 = 4.0;
const MAX_STAR_LUMINANCE: f32 = 400.0;

#[derive(Debug, Component)]
// Star mesh and materials for each levels of luminance
struct StarResources {
    mesh:      Handle<Mesh>,
    materials: [Handle<StandardMaterial>; LUMINANCE_LEVELS],
}

struct StarInfo {
    speed:     f32,
    lum_level: usize,
    transform: Transform,
}

fn generate_star(rng: &mut Entropy<WyRand>, count: usize) -> StarInfo {
    const STARBOX_RADIUS: f32 = 10.0;

    // Star's angle on the half-cylinder skybox
    let angle = random_range(rng, 0.0, PI);

    // Static stars
    let speed = match count < BACK_STAR_AMOUNT {
        true => 0.0,
        false => random_range(rng, MIN_STAR_SPEED.sqrt(), MAX_STAR_SPEED.sqrt()).powi(2),
    };

    // Initial stars
    let height = match count < MAX_STAR_AMOUNT {
        true => random_range(rng, MIN_STAR_HEIGHT, MAX_STAR_HEIGHT),
        false => MAX_STAR_HEIGHT,
    };

    let lum_level = rng.next_u32() as usize % 4;

    let transform = Transform::from_xyz(
        STARBOX_RADIUS * angle.cos(),
        height,
        STARBOX_RADIUS * angle.sin(),
    )
    .with_rotation(Quat::from_rotation_y(0.75 * TAU - angle));

    StarInfo {
        speed,
        lum_level,
        transform,
    }
}

fn setup_stars(
    mut commands: Commands,
    mut rng: GlobalEntropy<WyRand>,
    asset_server: Res<AssetServer>,
) {
    info!("Spawning stars");

    let lum_increment = (MAX_STAR_LUMINANCE - MIN_STAR_LUMINANCE) / LUMINANCE_LEVELS as f32;
    let resources = StarResources {
        mesh:      asset_server.add(Circle::new(0.01).into()),
        materials: (0..LUMINANCE_LEVELS)
            .map(|level| {
                let lum = lum_increment * level as f32;
                asset_server.add(StandardMaterial {
                    emissive: LinearRgba::rgb(lum, lum, lum),
                    ..default()
                })
            })
            .collect::<Vec<Handle<StandardMaterial>>>()
            .try_into()
            .unwrap(),
    };

    commands
        .spawn((
            OnEggScene,
            StarRoot,
            Name::new("Stars"),
            Transform::default(),
            Visibility::default(),
        ))
        .with_children(|parent| {
            (0..MAX_STAR_AMOUNT)
                .map(|i| generate_star(&mut rng, i))
                .for_each(|info| {
                    let StarInfo {
                        speed,
                        lum_level,
                        transform,
                    } = info;
                    parent.spawn((
                        Star { speed },
                        Mesh3d(resources.mesh.clone_weak()),
                        MeshMaterial3d(resources.materials[lum_level].clone_weak()),
                        transform,
                    ));
                })
        })
        .insert(resources);
}

fn update_stars_position(mut stars: Query<(&Star, &mut Transform)>) {
    stars
        .iter_mut()
        .for_each(|(Star { speed }, mut transform)| {
            transform.translation.y -= speed;
        });
}

fn despawn_respawn_stars(
    stars: Query<(Entity, &Transform), With<Star>>,
    star_root: Single<(Entity, &StarResources), With<StarRoot>>,
    mut rng: GlobalEntropy<WyRand>,
    mut commands: Commands,
) {
    let (star_root, resources) = star_root.into_inner();
    stars
        .iter()
        .filter_map(|(entity, transform)| {
            (transform.translation.y <= MIN_STAR_HEIGHT).then_some(entity)
        })
        .for_each(|entity| {
            commands.entity(entity).despawn();
            let StarInfo {
                speed,
                lum_level,
                transform,
            } = generate_star(&mut rng, MAX_STAR_AMOUNT);
            commands.entity(star_root).with_child((
                Star { speed },
                Mesh3d(resources.mesh.clone_weak()),
                MeshMaterial3d(resources.materials[lum_level].clone_weak()),
                transform,
            ));
        });
}

#[derive(Debug, Deref, Component)]
struct ExitNode(AnimationNodeIndex);

fn setup_ease_and_play(
    mut commands: Commands,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut animation_clips: ResMut<Assets<AnimationClip>>,
    player: Single<(Entity, &Transform), With<Player>>,
) {
    const EASE_DURATION: f32 = 3.0;
    const SPECIAL_FRAME_TRANSLATION: Vec3 = vec3(1.0, 1.0, 0.5);
    const SPECIAL_FRAME_ROTATION: Quat =
        Quat::from_array([0.0, FRAC_1_SQRT_2, 0.0, -FRAC_1_SQRT_2]);
    const CRACK_FRAME_TRANSLATION: Vec3 = vec3(1.49, 1.0, 0.5);

    let (player_entity, player_transform) = player.into_inner();

    let player_target_name = Name::new("Player");
    let player_target_id = AnimationTargetId::from_name(&player_target_name);
    let animation_domain = interval(0.0, EASE_DURATION).unwrap();

    let (animation_graph, animation_node_index) = AnimationGraph::from_clips([
        animation_clips.add({
            let mut ease_into_frame_clip = AnimationClip::default();
            ease_into_frame_clip.add_curve_to_target(
                player_target_id,
                AnimatableCurve::new(
                    animated_field!(Transform::translation),
                    EasingCurve::new(
                        player_transform.translation,
                        SPECIAL_FRAME_TRANSLATION,
                        EaseFunction::ExponentialInOut,
                    )
                    .reparametrize_linear(animation_domain)
                    .unwrap(),
                ),
            );
            ease_into_frame_clip.add_curve_to_target(
                player_target_id,
                AnimatableCurve::new(
                    animated_field!(Transform::rotation),
                    EasingCurve::new(
                        player_transform.rotation,
                        SPECIAL_FRAME_ROTATION,
                        EaseFunction::ExponentialInOut,
                    )
                    .reparametrize_linear(animation_domain)
                    .unwrap(),
                ),
            );
            ease_into_frame_clip
        }),
        animation_clips.add({
            let mut ease_into_crack_clip = AnimationClip::default();
            ease_into_crack_clip.add_curve_to_target(
                player_target_id,
                AnimatableCurve::new(
                    animated_field!(Transform::translation),
                    EasingCurve::new(
                        SPECIAL_FRAME_TRANSLATION,
                        CRACK_FRAME_TRANSLATION,
                        EaseFunction::ExponentialInOut,
                    )
                    .reparametrize_linear(animation_domain)
                    .unwrap(),
                ),
            );
            ease_into_crack_clip.add_event_fn(0.0, |commands, _entity, _time, _weight| {
                commands.run_system_cached(effects::fade_to_white);
            });
            ease_into_crack_clip
        }),
    ]);
    let animation_graph_handle = animation_graphs.add(animation_graph);

    let mut animation_player = AnimationPlayer::default();
    animation_player.play(animation_node_index[0]);

    commands.entity(player_entity).insert((
        animation_player,
        ExitNode(animation_node_index[1]),
        AnimationGraphHandle(animation_graph_handle),
        AnimationTarget {
            id:     player_target_id,
            player: player_entity,
        },
    ));
}

fn wait_for_ease(
    player_animation: Single<&AnimationPlayer, With<Player>>,
    mut q_elements: Query<(&CrackingElementInfo, &mut AnimationPlayer), Without<Player>>,
    mut crack_phase: ResMut<NextState<CrackingPhase>>,
) {
    if player_animation.all_finished() {
        if let Some((info, mut animator)) = q_elements
            .iter_mut()
            .find(|(info, _animator)| matches!(info.id, CrackingElementId::PunchLower))
        {
            animator.play(info.in_node);
            crack_phase.set(CrackingPhase::Punch);
        }
    }
}

fn advance_crack_phase(
    crack_phase: Res<State<CrackingPhase>>,
    mut next_crack_phase: ResMut<NextState<CrackingPhase>>,
    mut q_elements: Query<(&CrackingElementInfo, &mut AnimationPlayer), Without<Player>>,
) {
    match crack_phase.get() {
        CrackingPhase::Punch => next_crack_phase.set(CrackingPhase::FastPunch),
        CrackingPhase::FastPunch => {
            if let Some((info, mut animator)) = q_elements
                .iter_mut()
                .find(|(info, _animator)| matches!(info.id, CrackingElementId::PunchUpper))
            {
                animator.play(info.in_node);
            }
            next_crack_phase.set(CrackingPhase::QuadPunch)
        }
        CrackingPhase::QuadPunch => next_crack_phase.set(CrackingPhase::Violence),
        _ => (),
    }
}

fn punch(
    user_input: Res<UserInput>,
    q_elements: Query<&CrackingElementInfo, Without<Player>>,
    mut q_sprite_animations: Query<&mut SpriteAnimation>,
    mut right_left: Local<bool>,
) {
    if let Some(info) = q_elements
        .iter()
        .find(|info| matches!(info.id, CrackingElementId::PunchLower))
    {
        let current_fist = info.parts[*right_left as usize];
        if let Ok(mut animation) = q_sprite_animations.get_mut(current_fist) {
            match user_input.interact {
                KeyState::Press => *animation = SpriteAnimation::set_frame(1),
                KeyState::Release => {
                    *animation = SpriteAnimation::set_frame(0);
                    *right_left ^= true;
                }
                _ => (),
            }
        }
    }
}

fn fast_punch(
    user_input: Res<UserInput>,
    q_elements: Query<&CrackingElementInfo, Without<Player>>,
    mut q_sprite_animations: Query<&mut SpriteAnimation>,
    mut right_left: Local<bool>,
) {
    const FLIP: [bool; 2] = [false, true];

    if let Some(info) = q_elements
        .iter()
        .find(|info| info.id == CrackingElementId::PunchLower)
    {
        info.parts.iter().zip(FLIP).for_each(|(&entity, flip)| {
            if let Ok(mut animation) = q_sprite_animations.get_mut(entity) {
                match user_input.interact {
                    KeyState::Press => {
                        let toggle = match *right_left ^ flip {
                            true => 1.0,
                            false => 0.0,
                        };
                        *animation = SpriteAnimation::new(0, 1, 12)
                            .with_delay(0.083 * toggle)
                            .looping();
                    }
                    KeyState::Release => {
                        *animation = SpriteAnimation::set_frame(0);
                        *right_left ^= true;
                    }
                    _ => (),
                }
            }
        });
    }
}

fn quad_punch(
    user_input: Res<UserInput>,
    q_elements: Query<&CrackingElementInfo, Without<Player>>,
    mut q_sprite_animations: Query<&mut SpriteAnimation>,
) {
    q_elements
        .iter()
        .filter_map(|info| {
            matches!(
                info.id,
                CrackingElementId::PunchLower | CrackingElementId::PunchUpper
            )
            .then_some(&info.parts)
        })
        .flatten()
        .enumerate()
        .for_each(|(i, &entity)| {
            if let Ok(mut animation) = q_sprite_animations.get_mut(entity) {
                match user_input.interact {
                    KeyState::Press => {
                        *animation = SpriteAnimation::new(0, 1, 16)
                            .with_delay(0.016 * i as f32)
                            .looping()
                    }
                    KeyState::Release => *animation = SpriteAnimation::set_frame(0),
                    _ => (),
                }
            }
        });
}

fn violence(
    user_input: Res<UserInput>,
    mut q_elements: Query<(&CrackingElementInfo, &mut AnimationPlayer), Without<Player>>,
    mut q_sprite_animations: Query<&mut SpriteAnimation>,
) {
    struct SpriteInfo {
        len: usize,
        fps: u8,
    }

    const GUNS_SPRITE_ANIMATION_INFO: [SpriteInfo; 4] = [
        SpriteInfo { len: 4, fps: 60 },
        SpriteInfo { len: 15, fps: 24 },
        SpriteInfo { len: 4, fps: 24 },
        SpriteInfo { len: 4, fps: 12 },
    ];

    q_elements
        .iter_mut()
        .for_each(|(info, mut animator)| match info.id {
            CrackingElementId::Guns => match user_input.interact {
                KeyState::Press => {
                    animator.stop_all().play(info.in_node);
                    info.parts.iter().zip(GUNS_SPRITE_ANIMATION_INFO).for_each(
                        |(&entity, SpriteInfo { len, fps })| {
                            if let Ok(mut animation) = q_sprite_animations.get_mut(entity) {
                                *animation = SpriteAnimation::new(0, len - 1, fps).looping();
                            }
                        },
                    );
                }
                KeyState::Release => {
                    animator.stop_all().play(info.out_node);
                    info.parts.iter().for_each(|&entity| {
                        if let Ok(mut animation) = q_sprite_animations.get_mut(entity) {
                            *animation = SpriteAnimation::set_frame(0);
                        }
                    });
                }
                _ => (),
            },
            // Upper & Lower Punches
            _ => match user_input.interact {
                KeyState::Press => {
                    animator.stop_all().play(info.out_node);
                }
                KeyState::Release => {
                    animator.stop_all().play(info.in_node);
                }
                _ => (),
            },
        });
}

fn exit_egg(
    crack_health: Single<&Health, With<Crack>>,
    mut q_elements: Query<(&CrackingElementInfo, &mut AnimationPlayer), Without<Player>>,
    mut q_sprite_animations: Query<&mut SpriteAnimation>,
    player: Single<(&mut AnimationPlayer, &ExitNode), With<Player>>,
    mut crack_phase: ResMut<NextState<CrackingPhase>>,
) {
    if !crack_health.eq(&u8::MIN) {
        return;
    }

    q_elements
        .iter_mut()
        .filter(|(info, animator)| animator.is_playing_animation(info.in_node))
        .for_each(|(info, mut animator)| {
            animator.stop_all().play(info.out_node);
            info.parts.iter().for_each(|&entity| {
                if let Ok(mut animation) = q_sprite_animations.get_mut(entity) {
                    *animation = SpriteAnimation::set_frame(0);
                }
            });
        });

    let (mut player_animation, exit_node) = player.into_inner();
    player_animation.stop_all().play(exit_node.0);

    crack_phase.set(CrackingPhase::Fading);
}

fn exit_wait_for_fade(
    player_animation: Single<&AnimationPlayer, With<Player>>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    if player_animation.all_finished() {
        game_state.set(GameState::TopDown);
    }
}

fn update_crack(
    user_input: Res<UserInput>,
    q_element_info: Query<&CrackingElementInfo>,
    crack: Single<
        (
            &mut Health,
            &mut MeshMaterial3d<StandardMaterial>,
            &CrackMaterials,
        ),
        With<Crack>,
    >,
    mut e_reader: EventReader<SpriteAnimationFinished>,
    mut elements: Local<Vec<Entity>>,
) {
    if !matches!(user_input.interact, KeyState::Press | KeyState::Hold) || e_reader.is_empty() {
        return;
    }

    if elements.is_empty() {
        elements.extend(q_element_info.iter().flat_map(|info| info.parts.clone()));
    }

    let (mut crack_health, mut current_material, crack_materials) = crack.into_inner();

    let old_damage_level = damage_level(crack_health.0);

    let damage = e_reader
        .read()
        .filter(|event| elements.contains(&event.0))
        .count() as u8;
    e_reader.clear();

    crack_health.0 = crack_health.saturating_sub(damage);

    let new_damage_level = damage_level(crack_health.0);

    if new_damage_level != old_damage_level {
        current_material.0 = crack_materials[new_damage_level].clone_weak();
    }

    fn damage_level(health: u8) -> usize {
        match health {
            255 => 0,
            192..255 => 1,
            128..192 => 2,
            64..128 => 3,
            1..64 => 4,
            0 => 5,
        }
    }
}

fn over_interactables(
    trigger: Trigger<Pointer<Over>>,
    q_interactables: Query<Entity, With<EntityInteraction>>,
    mut interaction_target: Single<&mut InteractTarget, With<Player>>,
) {
    if trigger.pointer_id.is_mouse() {
        // Disregard mouse inputs
        return;
    }

    // info!("Hovering");
    if let Ok(target_entity) = q_interactables.get(trigger.target()) {
        // info!("Over Target: {}", target_entity);
        interaction_target.set(target_entity);
    }
}

fn out_interactables(
    trigger: Trigger<Pointer<Out>>,
    q_interactables: Query<Entity, With<EntityInteraction>>,
    mut interaction_target: Single<&mut InteractTarget, With<Player>>,
) {
    if trigger.pointer_id.is_mouse() {
        // Disregard mouse inputs
        return;
    }

    // info!("Not Hovering");
    if let Ok(_target_entity) = q_interactables.get(trigger.target()) {
        // info!("Out Target: {}", target_entity);
        interaction_target.clear();
    }
}

use bevy::picking::backend::PointerHits;
use bevy::picking::pointer::PointerId;
fn get_egg_interactions(
    interact_target: Single<&InteractTarget, With<Player>>,
    mut pointer_hits: EventReader<PointerHits>,
    egg_pointer_id: Single<&PointerId, With<EggPointer>>,
    q_interactables: Query<&EntityInteraction>,
) -> Option<EntityInteraction> {
    const INTERACTION_RANGE: f32 = 1.0;

    let target_entity = interact_target.get()?;

    let current_frame_hit = pointer_hits
        .read()
        .filter(|hit| &hit.pointer == *egg_pointer_id && hit.order == 0.0)
        .last()?;

    let hitdata = current_frame_hit
        .picks
        .iter()
        .find_map(|(entity, hitdata)| (entity == target_entity).then_some(hitdata))?;

    let target_in_range = hitdata.depth < INTERACTION_RANGE;

    target_in_range.then(|| q_interactables.get(*target_entity).ok().cloned())?
}
