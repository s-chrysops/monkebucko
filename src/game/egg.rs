use std::f32::consts::*;

use bevy::{
    animation::*,
    color::palettes::css::*,
    core_pipeline::{bloom::Bloom, tonemapping::Tonemapping},
    input::mouse::AccumulatedMouseMotion,
    math::u8,
    prelude::*,
};
use bevy_rand::prelude::*;
use rand_core::RngCore;

use crate::{
    RENDER_LAYER_OVERLAY, RENDER_LAYER_WORLD,
    animation::{SpriteAnimation, SpriteAnimationFinished},
    auto_scaling::AspectRatio,
    despawn_screen,
    game::interactions::*,
};

use super::*;

#[derive(Debug, Component)]
struct OnEggScene;

#[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[source(GameState = GameState::Egg)]
enum EggState {
    #[default]
    None,
    Special,
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
            spawn_player,
            spawn_world,
            spawn_stars,
            spawn_egg_special_elements,
            cursor_grab,
        ),
    )
    .add_systems(
        OnExit(GameState::Egg),
        (
            despawn_screen::<OnEggScene>,
            cursor_ungrab,
            effects::fade_from_white,
        ),
    )
    .add_systems(
        Update,
        (
            move_stars,
            (
                move_player,
                get_egg_interactions
                    .pipe(play_interactions)
                    .run_if(just_pressed_interact),
            )
                .run_if(in_state(MovementState::Enabled)),
        )
            .run_if(in_state(GameState::Egg)),
    )
    .add_systems(OnEnter(EggState::Special), setup_camera_movements)
    .add_systems(
        Update,
        (egg_special, update_crack).run_if(in_state(EggState::Special)),
    )
    .add_sub_state::<EggState>()
    .init_resource::<StarResources>();
}

// #[derive(Debug, Component, Deref, DerefMut)]
// struct Velocity(Vec3);

fn spawn_player(mut commands: Commands) {
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

const CRACK_HEALTH: u8 = 255;

#[derive(Debug, Deref, Component)]
struct CrackMaterials([Handle<StandardMaterial>; 6]);

fn spawn_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    info!("Spawning egg world");
    let floor = meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(1.5)));
    let ceiling = meshes.add(Plane3d::new(Vec3::NEG_Y, Vec2::splat(1.5)));

    let wall_west = meshes.add(Plane3d::new(Vec3::X, Vec2::splat(1.5)));
    let wall_east = meshes.add(Plane3d::new(Vec3::NEG_X, Vec2::splat(1.5)));
    let wall_north = meshes.add(Plane3d::new(Vec3::Z, Vec2::splat(1.5)));

    let wall_south_upper = meshes.add(Cuboid::new(3.0, 1.0, 0.05));
    let wall_south_lower = meshes.add(Cuboid::new(3.0, 0.8, 0.05));
    let wall_south_right = meshes.add(Cuboid::new(0.8, 1.2, 0.05));
    let wall_south_left = meshes.add(Cuboid::new(0.2, 1.2, 0.05));
    let window_sill = meshes.add(Cuboid::new(2.0, 0.01, 0.1));

    let bed = meshes.add(Cuboid::new(2.4, 0.3, 1.0));

    let material = materials.add(Color::WHITE);

    let room_elements = [
        (ceiling, (0.0, 3.0, 0.0)),
        (floor, (0.0, 0.0, 0.0)),
        (wall_west, (-1.5, 1.5, 0.0)),
        (wall_east, (1.5, 1.5, 0.0)),
        (wall_north, (0.0, 1.5, -1.5)),
        (wall_south_upper, (0.0, 2.5, 1.525)),
        (wall_south_lower, (0.0, 0.4, 1.525)),
        (wall_south_right, (-1.1, 1.4, 1.525)),
        (wall_south_left, (1.4, 1.4, 1.525)),
        (window_sill, (0.3, 0.8, 1.5)),
    ];

    commands.spawn_batch(room_elements.map(|(mesh, (x, y, z))| {
        (
            OnEggScene,
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(x, y, z),
        )
    }));

    // Window Glass
    commands
        .spawn((
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
        ))
        .observe(over_interactables)
        .observe(out_interactables);

    commands.spawn((
        OnEggScene,
        Mesh3d(bed),
        MeshMaterial3d(material.clone()),
        Transform::from_xyz(-0.3, 0.16, -1.0),
    ));

    // Star light
    commands.spawn((
        OnEggScene,
        PointLight {
            color: Color::from(LAVENDER),
            intensity: 10000.0,
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
        materials.add(StandardMaterial {
            base_color_texture: Some(asset_server.load(path)),
            perceptual_roughness: 1.0,
            alpha_mode: AlphaMode::Mask(0.5),
            cull_mode: None,
            emissive: LinearRgba::rgb(150.0, 150.0, 150.0),
            ..default()
        })
    }));

    // Crack
    let crack_entity = commands.spawn_empty().id();
    commands
        .entity(crack_entity)
        .insert((
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
                commands.set_state(EggState::Special);
            }),
        ))
        .observe(over_interactables)
        .observe(out_interactables);
}

const ROOM_BOUNDARY: Vec3 = Vec3::splat(1.3);
const PLAYER_STEP: f32 = 0.04;

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
    }
}

#[derive(Debug, PartialEq)]
enum EggSpecialElementId {
    PunchLower,
    PunchUpper,
    Guns,
}

#[derive(Debug, Component)]
struct EggSpecialElementInfo {
    id:       EggSpecialElementId,
    parts:    Vec<Entity>,
    in_node:  AnimationNodeIndex,
    out_node: AnimationNodeIndex,
}

fn spawn_egg_special_elements(mut commands: Commands, asset_server: Res<AssetServer>) {
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
                EggSpecialElementInfo {
                    id:       EggSpecialElementId::PunchLower,
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
                EggSpecialElementInfo {
                    id:       EggSpecialElementId::PunchUpper,
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
            EggSpecialElementInfo {
                id:       EggSpecialElementId::Guns,
                parts:    elements,
                in_node:  guns_nodes[0],
                out_node: guns_nodes[1],
            },
            AnimationGraphHandle(guns_graph_handle),
        ));
    }
}

const MAX_STAR_AMOUNT: usize = 300;
const BACK_STAR_AMOUNT: usize = 200;
const MIN_STAR_HEIGHT: f32 = -15.0;
const MAX_STAR_HEIGHT: f32 = 30.0;
const LUMINANCE_LEVELS: usize = 4;
const MIN_STAR_LUMINANCE: f32 = 4.0;
const MAX_STAR_LUMINANCE: f32 = 400.0;
const MIN_STAR_SPEED: f32 = 0.01;
const MAX_STAR_SPEED: f32 = 0.5;

// Star with parallax speed
#[derive(Debug, Component)]
struct Star(f32);

// Star mesh and materials with 4 levels of luminance
#[derive(Debug, Resource)]
struct StarResources {
    mesh:      Handle<Mesh>,
    materials: [Handle<StandardMaterial>; LUMINANCE_LEVELS],
}

impl FromWorld for StarResources {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let mesh = meshes.add(Circle::new(0.01));

        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let lum_increment = (MAX_STAR_LUMINANCE - MIN_STAR_LUMINANCE) / LUMINANCE_LEVELS as f32;
        let materials = (0..LUMINANCE_LEVELS)
            .map(|level| {
                let lum = lum_increment * level as f32;
                materials.add(StandardMaterial {
                    emissive: LinearRgba::rgb(lum, lum, lum),
                    ..default()
                })
            })
            .collect::<Vec<Handle<StandardMaterial>>>()
            .try_into()
            .unwrap();

        StarResources { mesh, materials }
    }
}

fn generate_star(rng: &mut Entropy<WyRand>, count: usize) -> (f32, usize, Transform) {
    // Star's angle on the half-cylinder skybox
    let angle = random_range(rng, 0.0, std::f32::consts::PI);

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

    let lum = rng.next_u32() as usize % 4;

    let mut transform = Transform::from_xyz(10.0 * ops::cos(angle), height, 10.0 * ops::sin(angle));
    transform.rotate_local_y(3.0 * std::f32::consts::FRAC_PI_2 - angle);

    (speed, lum, transform)
}

fn spawn_stars(
    mut commands: Commands,
    mut rng: GlobalEntropy<WyRand>,
    resources: Res<StarResources>,
) {
    info!("Spawning stars");
    let initial_stars: Vec<_> = (0..MAX_STAR_AMOUNT)
        .map(|i| generate_star(&mut rng, i))
        .map(|(speed, lum, transform)| {
            (
                OnEggScene,
                Star(speed),
                Mesh3d(resources.mesh.clone_weak()),
                MeshMaterial3d(resources.materials[lum].clone_weak()),
                transform,
            )
        })
        .collect();
    commands.spawn_batch(initial_stars);
}

fn move_stars(
    mut commands: Commands,
    stars: Query<(Entity, &Star, &mut Transform)>,
    mut rng: GlobalEntropy<WyRand>,
    resources: Res<StarResources>,
) {
    stars
        .into_iter()
        .for_each(|(entity, Star(speed), mut transform)| {
            transform.translation.y -= speed;
            if transform.translation.y <= MIN_STAR_HEIGHT {
                commands.entity(entity).despawn();
                let (speed, lum, transform) = generate_star(&mut rng, MAX_STAR_AMOUNT);
                commands.spawn((
                    OnEggScene,
                    Star(speed),
                    Mesh3d(resources.mesh.clone_weak()),
                    MeshMaterial3d(resources.materials[lum].clone_weak()),
                    transform,
                ));
            }
        });
}

#[derive(Debug, Deref, Component)]
struct ExitNode(AnimationNodeIndex);

const EASE_DURATION: f32 = 3.0;
const SPECIAL_FRAME_TRANSLATION: Vec3 = vec3(1.0, 1.0, 0.5);
const SPECIAL_FRAME_ROTATION: Quat = Quat::from_array([0.0, FRAC_1_SQRT_2, 0.0, -FRAC_1_SQRT_2]);
const CRACK_FRAME_TRANSLATION: Vec3 = vec3(1.49, 1.0, 0.5);

fn setup_camera_movements(
    mut commands: Commands,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut animation_clips: ResMut<Assets<AnimationClip>>,
    player: Single<(Entity, &Transform), With<Player>>,
) {
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

#[derive(Debug, Default)]
enum EggSpecialState {
    #[default]
    Easing,
    Punch,
    PunchRapid,
    MorePunchRapid,
    Violence,
    Fading,
}

#[allow(clippy::too_many_arguments)] // lmao
fn egg_special(
    mut commands: Commands,
    player: Single<(&mut AnimationPlayer, &ExitNode), With<Player>>,
    mut controllers: Query<(&EggSpecialElementInfo, &mut AnimationPlayer), Without<Player>>,
    mut q_sprite_animations: Query<&mut SpriteAnimation>,
    crack: Single<&Health, With<Crack>>,
    user_input: Res<UserInput>,
    mut state: Local<EggSpecialState>,
    mut right_left: Local<bool>,
) {
    let (mut player_animation, exit_node) = player.into_inner();

    match *state {
        EggSpecialState::Easing => {
            if player_animation.all_finished() {
                let (info, mut player) = controllers
                    .iter_mut()
                    .find(|(info, _player)| info.id == EggSpecialElementId::PunchLower)
                    .unwrap();
                player.play(info.in_node);

                *state = EggSpecialState::Punch;
            }
        }
        EggSpecialState::Punch => {
            if matches!(user_input.interact, KeyState::Press) {
                let (info, _player) = controllers
                    .iter()
                    .find(|(info, _player)| info.id == EggSpecialElementId::PunchLower)
                    .unwrap();

                *q_sprite_animations
                    .get_mut(info.parts[*right_left as usize])
                    .unwrap() = SpriteAnimation::set_frame(1);
            } else if matches!(user_input.interact, KeyState::Release)
                || matches!(user_input.swap, KeyState::Press)
            {
                let (info, _player) = controllers
                    .iter()
                    .find(|(info, _player)| info.id == EggSpecialElementId::PunchLower)
                    .unwrap();

                *q_sprite_animations
                    .get_mut(info.parts[*right_left as usize])
                    .unwrap() = SpriteAnimation::set_frame(0);

                *right_left ^= true;
            }

            if matches!(user_input.swap, KeyState::Press) {
                *state = EggSpecialState::PunchRapid;
            }
        }
        EggSpecialState::PunchRapid => {
            if matches!(user_input.interact, KeyState::Press) {
                let (info, _player) = controllers
                    .iter()
                    .find(|(info, _player)| info.id == EggSpecialElementId::PunchLower)
                    .unwrap();

                info.parts
                    .iter()
                    .zip([false, true])
                    .for_each(|(&entity, flip)| {
                        let toggle = match *right_left ^ flip {
                            true => 1.0,
                            false => 0.0,
                        };
                        *q_sprite_animations.get_mut(entity).unwrap() =
                            SpriteAnimation::new(0, 1, 12)
                                .with_delay(0.083 * toggle)
                                .looping();
                    });
            } else if matches!(user_input.interact, KeyState::Release)
                || matches!(user_input.swap, KeyState::Press)
            {
                let (info, _player) = controllers
                    .iter()
                    .find(|(info, _player)| info.id == EggSpecialElementId::PunchLower)
                    .unwrap();

                info.parts.iter().for_each(|&entity| {
                    *q_sprite_animations.get_mut(entity).unwrap() = SpriteAnimation::set_frame(0);
                });

                *right_left ^= true;
            }

            if matches!(user_input.swap, KeyState::Press) {
                let (info, mut player) = controllers
                    .iter_mut()
                    .find(|(info, _)| info.id == EggSpecialElementId::PunchUpper)
                    .unwrap();
                player.play(info.in_node);

                *state = EggSpecialState::MorePunchRapid;
            }
        }
        EggSpecialState::MorePunchRapid => {
            if matches!(user_input.interact, KeyState::Press) {
                controllers
                    .iter()
                    .filter_map(|(info, _player)| {
                        (info.id == EggSpecialElementId::PunchLower
                            || info.id == EggSpecialElementId::PunchUpper)
                            .then_some(&info.parts)
                    })
                    .flatten()
                    .enumerate()
                    .for_each(|(i, &entity)| {
                        *q_sprite_animations.get_mut(entity).unwrap() =
                            SpriteAnimation::new(0, 1, 16)
                                .with_delay(0.016 * i as f32)
                                .looping()
                    });
            } else if matches!(user_input.interact, KeyState::Release)
                || matches!(user_input.swap, KeyState::Press)
            {
                controllers
                    .iter()
                    .filter_map(|(info, _player)| {
                        (info.id == EggSpecialElementId::PunchLower
                            || info.id == EggSpecialElementId::PunchUpper)
                            .then_some(&info.parts)
                    })
                    .flatten()
                    .for_each(|&entity| {
                        *q_sprite_animations.get_mut(entity).unwrap() =
                            SpriteAnimation::set_frame(0);
                    });
            }

            if matches!(user_input.swap, KeyState::Press) {
                *state = EggSpecialState::Violence;
            }
        }
        EggSpecialState::Violence => {
            const GUNS_SPRITE_ANIMATION_INFO: [(usize, usize, u8); 4] =
                [(0, 3, 60), (0, 14, 24), (0, 3, 24), (0, 3, 12)];

            if matches!(user_input.interact, KeyState::Press) {
                controllers
                    .iter_mut()
                    .filter(|(info, _player)| {
                        info.id == EggSpecialElementId::PunchLower
                            || info.id == EggSpecialElementId::PunchUpper
                    })
                    .for_each(|(info, mut player)| {
                        player.stop_all().play(info.out_node);
                    });

                let (info, mut player) = controllers
                    .iter_mut()
                    .find(|(info, _player)| info.id == EggSpecialElementId::Guns)
                    .unwrap();
                player.stop_all().play(info.in_node);

                info.parts.iter().zip(GUNS_SPRITE_ANIMATION_INFO).for_each(
                    |(&entity, (first, last, fps))| {
                        *q_sprite_animations.get_mut(entity).unwrap() =
                            SpriteAnimation::new(first, last, fps).looping();
                    },
                );
            } else if matches!(user_input.interact, KeyState::Release) {
                controllers
                    .iter_mut()
                    .filter(|(info, _player)| {
                        info.id == EggSpecialElementId::PunchLower
                            || info.id == EggSpecialElementId::PunchUpper
                    })
                    .for_each(|(info, mut player)| {
                        player.stop_all().play(info.in_node);
                    });

                let (info, mut player) = controllers
                    .iter_mut()
                    .find(|(info, _player)| info.id == EggSpecialElementId::Guns)
                    .unwrap();
                player.stop_all().play(info.out_node);
                info.parts.iter().for_each(|&entity| {
                    *q_sprite_animations.get_mut(entity).unwrap() = SpriteAnimation::set_frame(0);
                });
            }
        }
        EggSpecialState::Fading => {
            if player_animation.all_finished() {
                commands.set_state(GameState::TopDown);
            }
        }
    }

    if crack.eq(&u8::MIN) && matches!(user_input.jump, KeyState::Press) {
        controllers
            .iter_mut()
            .filter(|(info, player)| player.is_playing_animation(info.in_node))
            .for_each(|(info, mut player)| {
                info.parts.iter().for_each(|&entity| {
                    *q_sprite_animations.get_mut(entity).unwrap() = SpriteAnimation::set_frame(0);
                });
                player.stop_all().play(info.out_node);
            });
        player_animation.stop_all().play(exit_node.0);
        *state = EggSpecialState::Fading;
    }
}

fn update_crack(
    crack: Single<
        (
            &mut Health,
            &mut MeshMaterial3d<StandardMaterial>,
            &CrackMaterials,
        ),
        With<Crack>,
    >,
    mut e_reader: EventReader<SpriteAnimationFinished>,
    user_input: Res<UserInput>,
) {
    if !matches!(user_input.interact, KeyState::Press | KeyState::Hold) || e_reader.is_empty() {
        return;
    }

    let (mut crack_health, mut current_material, crack_materials) = crack.into_inner();

    let old_damage_level = damage_level(crack_health.0);

    let damage = e_reader.read().count() as u8;
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
    over: Trigger<Pointer<Over>>,
    q_interactables: Query<Entity, With<EntityInteraction>>,
    mut interaction_target: Single<&mut InteractTarget, With<Player>>,
) {
    // info!("Hovering");
    if let Ok(target_entity) = q_interactables.get(over.target()) {
        // info!("Over Target: {}", target_entity);
        interaction_target.set(target_entity);
    }
    // let depth = over.event().event.hit.depth;
    // info!(depth);
}

fn out_interactables(
    out: Trigger<Pointer<Out>>,
    q_interactables: Query<Entity, With<EntityInteraction>>,
    mut interaction_target: Single<&mut InteractTarget, With<Player>>,
) {
    // info!("Not Hovering");
    if let Ok(_target_entity) = q_interactables.get(out.target()) {
        // info!("Out Target: {}", target_entity);
        interaction_target.clear();
    }
}

fn get_egg_interactions(
    player: Single<(&InteractTarget, &Transform), With<Player>>,
    q_interactables: Query<(&EntityInteraction, &Transform)>,
) -> Option<EntityInteraction> {
    const INTERACTION_RANGE: f32 = 1.0;

    let (interact_target, player_transform) = player.into_inner();
    let target_entity = interact_target.as_ref()?;

    let (entity_interaction, target_transform) = q_interactables.get_inner(*target_entity).ok()?;

    let player_in_range = player_transform
        .translation
        .distance(target_transform.translation)
        < INTERACTION_RANGE;

    player_in_range.then_some(entity_interaction.clone())
}
