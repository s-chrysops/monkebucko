use std::f32::consts::*;

use bevy::{animation::*, prelude::*};

use super::*;

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

pub fn egg_cracking_plugin(app: &mut App) {
    app.add_sub_state::<CrackingPhase>();

    app.add_systems(
        OnEnter(GameState::Egg),
        (setup_cracking_animations, setup_cracking_elements),
    )
    .add_systems(Update, reveal_crack.run_if(in_state(EggState::Ready)))
    .add_systems(
        OnEnter(EggState::Cracking),
        (setup_ease_and_play, disable_movement),
    )
    .add_systems(
        Update,
        (
            advance_crack_phase.run_if(not(pressing_interact).and(just_pressed_swap)),
            (update_crack, update_intro_player).run_if(pressing_interact),
            play_egg_exit.run_if(just_pressed_jump.and(has_progress_flag(ProgressFlag::CrackOpen))),
            (cracking_animations_out, enable_movement, escape_cracking).run_if(just_pressed_escape),
            play_sfx.run_if(just_pressed_interact),
            (kill_all_sound, kill_violence).run_if(not(pressing_interact)),
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
    .add_systems(
        OnExit(CrackingPhase::Easing),
        (
            play_egg_exit.run_if(has_progress_flag(ProgressFlag::CrackOpen)),
            punch_in.run_if(not(has_progress_flag(ProgressFlag::CrackOpen))),
        ),
    )
    .add_systems(
        OnEnter(CrackingPhase::Fading),
        (effects::fade_to_white, cracking_animations_out),
    );
}

#[derive(Debug, PartialEq)]
enum CrackingAnimationId {
    PunchLower,
    PunchUpper,
    Guns,
}

#[derive(Debug, Component)]
struct CrackingAnimationInfo {
    id:       CrackingAnimationId,
    parts:    Vec<Entity>,
    in_node:  AnimationNodeIndex,
    out_node: AnimationNodeIndex,
}

#[derive(Debug, Deref, DerefMut, Component)]
struct CrackHealth(u8);

impl CrackHealth {
    fn damage_level(&self) -> usize {
        match self.0 {
            255 => 0,
            192..255 => 1,
            128..192 => 2,
            64..128 => 3,
            1..64 => 4,
            0 => 5,
        }
    }
}

#[derive(Debug, Deref, DerefMut, Component)]
struct CrackingTimer(Timer);

#[derive(Debug, Deref, Component)]
struct CrackMaterials([Handle<StandardMaterial>; 6]);

#[derive(Debug, Component)]
struct CrackingRoot;

#[derive(Debug, Component)]
struct AmiIntroPlayer;

#[derive(Debug, Component)]
struct CrackingSFX {
    punches:    [Handle<AudioSource>; 4],
    fast_punch: Handle<AudioSource>,
    quad_punch: Handle<AudioSource>,
    violence:   Handle<AudioSource>,
}

#[derive(Debug, Component)]
struct ViolenceMusic;

fn setup_cracking_elements(
    mut commands: Commands,
    mut asset_tracker: ResMut<AssetTracker>,
    asset_server: Res<AssetServer>,
    progress: Res<Progress>,
    settings: Res<Persistent<Settings>>,
) {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

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

    let cracking_sfx = CrackingSFX {
        punches:    [
            "audio/sfx/punch_0.ogg",
            "audio/sfx/punch_1.ogg",
            "audio/sfx/punch_2.ogg",
            "audio/sfx/punch_3.ogg",
        ]
        .map(|path| {
            let sound = asset_server.load(path);
            asset_tracker.push(sound.clone_weak().untyped());
            sound
        }),
        fast_punch: {
            let sound = asset_server.load("audio/sfx/fast_punch.ogg");
            asset_tracker.push(sound.clone_weak().untyped());
            sound
        },
        quad_punch: {
            let sound = asset_server.load("audio/sfx/quad_punch.ogg");
            asset_tracker.push(sound.clone_weak().untyped());
            sound
        },
        violence:   {
            let sound = asset_server.load("audio/sfx/violence.ogg");
            asset_tracker.push(sound.clone_weak().untyped());
            sound
        },
    };

    let cracking_root = commands
        .spawn((
            OnEggScene,
            CrackingRoot,
            Name::new("Cracking Root"),
            Transform::default(),
            Visibility::Hidden,
            cracking_sfx,
            SpecialInteraction::new(move |commands: &mut Commands, _entity: Entity| {
                commands.set_state(EggState::Cracking);
            }),
        ))
        .id();

    let ami_intro = asset_server.load("sprites/ami_intro.png");
    asset_tracker.push(ami_intro.clone_weak().untyped());
    let ami_layout = TextureAtlasLayout::from_grid(uvec2(128, 96), 10, 6, None, None);
    let ami_layout = asset_server.add(ami_layout);

    commands.spawn((
        ChildOf(cracking_root),
        Name::new("CRT_Sprite"),
        Sprite::from_atlas_image(ami_intro, ami_layout.into()),
        SpriteAnimation::new(0, 58, 12).looping(),
        Transform::default(),
        Visibility::default(),
        RENDER_LAYER_SPECIAL,
    ));

    let crack_materials = CrackMaterials(
        [
            "sprites/crack/crack1.png",
            "sprites/crack/crack2.png",
            "sprites/crack/crack3.png",
            "sprites/crack/crack4.png",
            "sprites/crack/crack5.png",
            "sprites/crack/crack6.png",
        ]
        .map(|path| {
            let image = asset_server.load(path);
            asset_tracker.push(image.clone_weak().untyped());
            asset_server.add(StandardMaterial {
                base_color_texture: Some(image),
                perceptual_roughness: 1.0,
                alpha_mode: AlphaMode::Mask(0.5),
                cull_mode: None,
                emissive: LinearRgba::rgb(150.0, 150.0, 150.0),
                ..default()
            })
        }),
    );

    let (health, Some(material), reveal_time) = (match progress.contains(&ProgressFlag::CrackOpen) {
        true => (0, crack_materials.last(), 2.0),
        false => (200, crack_materials.first(), 8.0),
    }) else {
        unreachable!()
    };

    let crack_transform =
        Transform::from_xyz(1.49, 1.0, 0.5).with_rotation(Quat::from_rotation_y(-FRAC_PI_2));

    // Crack
    commands.spawn((
        ChildOf(cracking_root),
        Name::new("Crack"),
        CrackHealth(health),
        crack_transform,
        Visibility::default(),
        Mesh3d(asset_server.add(Rectangle::new(1.0, 1.0).into())),
        MeshMaterial3d(material.clone_weak()),
        crack_materials,
        PICKABLE,
        EntityInteraction::Special(cracking_root),
    ));

    let violence_music = asset_server.load("audio/music/catching_air.ogg");
    asset_tracker.push(violence_music.clone_weak().untyped());
    commands.spawn((
        ViolenceMusic,
        ChildOf(cracking_root),
        Name::new("Violence Music"),
        AudioPlayer::new(violence_music),
        PlaybackSettings::LOOP
            .with_volume(Volume::Linear(settings.music_vol))
            .muted(),
    ));

    commands
        .entity(cracking_root)
        .insert(CrackingTimer(Timer::from_seconds(
            reveal_time,
            TimerMode::Once,
        )));

    let ami_intro = asset_server.load("audio/music/ami_intro.ogg");
    asset_tracker.push(ami_intro.clone_weak().untyped());

    commands.spawn((
        Music,
        AmiIntroPlayer,
        Name::new("Ami Intro Player"),
        AudioPlayer::new(ami_intro),
        PlaybackSettings::LOOP
            .paused()
            .with_volume(Volume::Linear(settings.music_vol / 6.0))
            .with_spatial(true),
        crack_transform,
    ));
}

fn setup_cracking_animations(mut commands: Commands, asset_server: Res<AssetServer>) {
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
                CrackingAnimationInfo {
                    id:       CrackingAnimationId::PunchLower,
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
                CrackingAnimationInfo {
                    id:       CrackingAnimationId::PunchUpper,
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

        struct PreAnimation {
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

        let pre_animations = [
            PreAnimation {
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
            PreAnimation {
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
            PreAnimation {
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
            PreAnimation {
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

        let (gun_animation_entities, in_out_curves): (
            Vec<Entity>,
            Vec<(
                AnimationTargetId,
                AnimatableCurve<_, _>,
                AnimatableCurve<_, _>,
            )>,
        ) = pre_animations
            .into_iter()
            .map(|pre_animation| {
                let PreAnimation {
                    name,
                    path,
                    layout,
                    flip_x,
                    custom_size,
                    rotation,
                    translation_in,
                    translation_out,
                } = pre_animation;

                let name = Name::new(name);
                let target_id = AnimationTargetId::from_name(&name);

                let animation_entity = commands
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

                (animation_entity, in_out_curves)
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

        commands
            .entity(guns)
            .add_children(&gun_animation_entities)
            .insert((
                CrackingAnimationInfo {
                    id:       CrackingAnimationId::Guns,
                    parts:    gun_animation_entities,
                    in_node:  guns_nodes[0],
                    out_node: guns_nodes[1],
                },
                AnimationGraphHandle(guns_graph_handle),
            ));
    }
}

fn reveal_crack(
    crack: Single<(&mut CrackingTimer, &mut Visibility), With<CrackingRoot>>,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    crt_panel: Single<&MeshMaterial3d<StandardMaterial>, With<CrtPanel>>,
    ami_intro: Single<&SpatialAudioSink, With<AmiIntroPlayer>>,
) {
    let (mut timer, mut visibility) = crack.into_inner();
    if timer.tick(time.delta()).just_finished() {
        info!("REVEAL");
        *visibility = Visibility::Visible;
        if let Some(material) = materials.get_mut(&crt_panel.0) {
            material.base_color = WHITE.into();
            material.emissive = LinearRgba::rgb(16.0, 16.0, 16.0);
        }
        ami_intro.play();
    }
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
    mut crack_phase: ResMut<NextState<CrackingPhase>>,
) {
    if player_animation.all_finished() {
        crack_phase.set(CrackingPhase::Punch);
    }
}

fn punch_in(mut q_elements: Query<(&CrackingAnimationInfo, &mut AnimationPlayer)>) {
    if let Some((info, mut animator)) = q_elements
        .iter_mut()
        .find(|(info, _animator)| matches!(info.id, CrackingAnimationId::PunchLower))
    {
        animator.stop_all().play(info.in_node);
    }
}

fn advance_crack_phase(
    crack_phase: Res<State<CrackingPhase>>,
    mut next_crack_phase: ResMut<NextState<CrackingPhase>>,
    mut q_elements: Query<(&CrackingAnimationInfo, &mut AnimationPlayer), Without<Player>>,
) {
    match crack_phase.get() {
        CrackingPhase::Punch => next_crack_phase.set(CrackingPhase::FastPunch),
        CrackingPhase::FastPunch => {
            if let Some((info, mut animator)) = q_elements
                .iter_mut()
                .find(|(info, _animator)| matches!(info.id, CrackingAnimationId::PunchUpper))
            {
                animator.stop_all().play(info.in_node);
            }
            next_crack_phase.set(CrackingPhase::QuadPunch)
        }
        CrackingPhase::QuadPunch => next_crack_phase.set(CrackingPhase::Violence),
        _ => (),
    }
}

fn punch(
    user_input: Res<UserInput>,
    q_elements: Query<&CrackingAnimationInfo, Without<Player>>,
    mut q_sprite_animations: Query<&mut SpriteAnimation>,
    mut right_left: Local<bool>,
) {
    if let Some(info) = q_elements
        .iter()
        .find(|info| matches!(info.id, CrackingAnimationId::PunchLower))
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
    q_elements: Query<&CrackingAnimationInfo, Without<Player>>,
    mut q_sprite_animations: Query<&mut SpriteAnimation>,
    mut right_left: Local<bool>,
) {
    const FLIP: [bool; 2] = [false, true];

    if let Some(info) = q_elements
        .iter()
        .find(|info| info.id == CrackingAnimationId::PunchLower)
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
    q_elements: Query<&CrackingAnimationInfo, Without<Player>>,
    mut q_sprite_animations: Query<&mut SpriteAnimation>,
) {
    q_elements
        .iter()
        .filter_map(|info| {
            matches!(
                info.id,
                CrackingAnimationId::PunchLower | CrackingAnimationId::PunchUpper
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
    mut q_elements: Query<(&CrackingAnimationInfo, &mut AnimationPlayer)>,
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
            CrackingAnimationId::Guns => match user_input.interact {
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

fn play_sfx(
    crack_phase: Res<State<CrackingPhase>>,
    sfx: Single<&CrackingSFX, With<CrackingRoot>>,
    settings: Res<Persistent<Settings>>,
    mut violence: Single<&mut AudioSink, With<ViolenceMusic>>,
    mut cursor: Local<usize>,
    mut commands: Commands,
) {
    match crack_phase.get() {
        CrackingPhase::Punch => {
            let Some(punch_sfx) = sfx.punches.get(*cursor) else {
                unreachable!();
            };

            commands.spawn((
                Sound,
                AudioPlayer::new(punch_sfx.clone_weak()),
                PlaybackSettings::DESPAWN.with_volume(Volume::Linear(settings.sound_vol)),
            ));

            *cursor += 1;
            if *cursor == sfx.punches.len() {
                *cursor = 0;
            }
        }
        CrackingPhase::FastPunch => {
            commands.spawn((
                Sound,
                AudioPlayer::new(sfx.fast_punch.clone_weak()),
                PlaybackSettings::LOOP.with_volume(Volume::Linear(settings.sound_vol)),
            ));
        }
        CrackingPhase::QuadPunch => {
            commands.spawn((
                Sound,
                AudioPlayer::new(sfx.quad_punch.clone_weak()),
                PlaybackSettings::LOOP.with_volume(Volume::Linear(settings.sound_vol)),
            ));
        }
        CrackingPhase::Violence => {
            violence.unmute();
            commands.spawn((
                Sound,
                AudioPlayer::new(sfx.violence.clone_weak()),
                PlaybackSettings::LOOP.with_volume(Volume::Linear(settings.sound_vol)),
            ));
        }
        _ => (),
    }
}

fn kill_violence(mut violence: Single<&mut AudioSink, With<ViolenceMusic>>) {
    violence.mute();
}

fn play_egg_exit(
    player: Single<(&mut AnimationPlayer, &ExitNode), With<Player>>,
    mut crack_phase: ResMut<NextState<CrackingPhase>>,
) {
    let (mut player_animation, ExitNode(node)) = player.into_inner();
    player_animation.stop_all().play(*node);

    crack_phase.set(CrackingPhase::Fading);
}

fn exit_wait_for_fade(
    player_animator: Single<&AnimationPlayer, With<Player>>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    if player_animator.all_finished() {
        game_state.set(GameState::TopDown);
    }
}

fn escape_cracking(
    mut player_animator: Single<&mut AnimationPlayer, With<Player>>,
    mut egg_state: ResMut<NextState<EggState>>,
) {
    player_animator.stop_all();
    egg_state.set(EggState::Ready);
}

fn cracking_animations_out(
    mut q_elements: Query<(&CrackingAnimationInfo, &mut AnimationPlayer), Without<Player>>,
    mut q_sprite_animations: Query<&mut SpriteAnimation>,
) {
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
}

fn update_crack(
    q_animation_info: Query<&CrackingAnimationInfo>,
    crack: Single<(
        &mut CrackHealth,
        &mut MeshMaterial3d<StandardMaterial>,
        &CrackMaterials,
    )>,
    mut e_reader: EventReader<SpriteAnimationFinished>,
    mut cracking_animations: Local<Vec<Entity>>,
    mut progress: ResMut<Progress>,
) {
    if e_reader.is_empty() {
        return;
    }

    if cracking_animations.is_empty() {
        // cache list of cracking sprite animations
        cracking_animations.extend(q_animation_info.iter().flat_map(|info| info.parts.clone()));
    }

    let (mut crack_health, mut current_material, crack_materials) = crack.into_inner();

    let old_damage_level = crack_health.damage_level();

    let damage = e_reader
        .read()
        .filter(|event| cracking_animations.contains(&event.entity))
        .count() as u8;
    e_reader.clear();

    crack_health.0 = crack_health.saturating_sub(damage);

    let new_damage_level = crack_health.damage_level();

    if new_damage_level != old_damage_level {
        if let Some(new_material) = crack_materials.get(new_damage_level) {
            current_material.0 = new_material.clone_weak();
        }
    }

    if new_damage_level == 5 {
        progress.insert(ProgressFlag::CrackOpen);
    }
}

fn update_intro_player(
    crack_health: Single<&CrackHealth, Changed<CrackHealth>>,
    mut ami_intro: Single<&mut SpatialAudioSink, With<AmiIntroPlayer>>,
    settings: Res<Persistent<Settings>>,
    mut old_damage_level: Local<usize>,
) {
    let new_damage_level = crack_health.damage_level();
    if *old_damage_level != new_damage_level {
        *old_damage_level = new_damage_level;
        let new_volume = Volume::Linear(settings.music_vol * (new_damage_level + 1) as f32 / 6.0);
        ami_intro.set_volume(new_volume);
    }
}
