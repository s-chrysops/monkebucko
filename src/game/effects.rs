use bevy::{animation::*, color::palettes::css::*, prelude::*};

use super::*;
use crate::{RENDER_LAYER_OVERLAY, WINDOW_HEIGHT, WINDOW_WIDTH};

pub fn effects_plugin(app: &mut App) {
    app.add_systems(Startup, (spawn_cinematic_bars, spawn_fades))
        .add_systems(Update, update_fade)
        .add_event::<FadeIn>()
        .add_event::<FadeOut>()
        .add_event::<CinematicBarsIn>()
        .add_event::<CinematicBarsOut>();
}

#[derive(Debug, Component)]
pub struct EffectsNodes {
    in_node:  AnimationNodeIndex,
    out_node: AnimationNodeIndex,
}

#[derive(Debug, Component)]
pub struct FadeBlack;

#[derive(Debug, Component)]
pub struct FadeWhite;

#[derive(Debug, Clone, Copy, Event)]
pub struct FadeIn;

#[derive(Debug, Clone, Copy, Event)]
pub struct FadeOut;

#[derive(Debug, Clone, Component, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct FadeOpacity(f32);

const FADE_DURATION: f32 = 2.0; // seconds

fn spawn_fades(mut commands: Commands, asset_server: Res<AssetServer>) {
    let fade_black_name = Name::new("Fade to Black");
    let fade_white_name = Name::new("Fade to White");

    let fade_black_target_id = AnimationTargetId::from_name(&fade_black_name);
    let fade_white_target_id = AnimationTargetId::from_name(&fade_white_name);

    let domain = interval(0.0, FADE_DURATION).unwrap();

    let (fade_black_graph, fade_black_nodes) = AnimationGraph::from_clips([
        asset_server.add({
            let mut fade_black_in_clip = AnimationClip::default();
            fade_black_in_clip.add_curve_to_target(
                fade_black_target_id,
                AnimatableCurve::new(
                    animated_field!(FadeOpacity::0),
                    EasingCurve::new(0.0, 1.0, EaseFunction::Linear)
                        .reparametrize_linear(domain)
                        .unwrap(),
                ),
            );
            fade_black_in_clip.add_event_fn(FADE_DURATION, |commands, _entity, _time, _weight| {
                commands.send_event(FadeIn);
            });
            fade_black_in_clip
        }),
        asset_server.add({
            let mut fade_black_out_clip = AnimationClip::default();
            fade_black_out_clip.add_curve_to_target(
                fade_black_target_id,
                AnimatableCurve::new(
                    animated_field!(FadeOpacity::0),
                    EasingCurve::new(1.0, 0.0, EaseFunction::Linear)
                        .reparametrize_linear(domain)
                        .unwrap(),
                ),
            );
            fade_black_out_clip.add_event_fn(FADE_DURATION, |commands, _entity, _time, _weight| {
                commands.send_event(FadeOut);
            });
            fade_black_out_clip
        }),
    ]);
    let (fade_white_graph, fade_white_nodes) = AnimationGraph::from_clips([
        asset_server.add({
            let mut fade_white_in_clip = AnimationClip::default();
            fade_white_in_clip.add_curve_to_target(
                fade_white_target_id,
                AnimatableCurve::new(
                    animated_field!(FadeOpacity::0),
                    EasingCurve::new(0.0, 1.0, EaseFunction::Linear)
                        .reparametrize_linear(domain)
                        .unwrap(),
                ),
            );
            fade_white_in_clip.add_event_fn(FADE_DURATION, |commands, _entity, _time, _weight| {
                commands.send_event(FadeIn);
            });
            fade_white_in_clip
        }),
        asset_server.add({
            let mut fade_white_out_clip = AnimationClip::default();
            fade_white_out_clip.add_curve_to_target(
                fade_white_target_id,
                AnimatableCurve::new(
                    animated_field!(FadeOpacity::0),
                    EasingCurve::new(1.0, 0.0, EaseFunction::Linear)
                        .reparametrize_linear(domain)
                        .unwrap(),
                ),
            );
            fade_white_out_clip.add_event_fn(FADE_DURATION, |commands, _entity, _time, _weight| {
                commands.send_event(FadeOut);
            });
            fade_white_out_clip
        }),
    ]);

    let fade_black_graph_handle = asset_server.add(fade_black_graph);
    let fade_white_graph_handle = asset_server.add(fade_white_graph);

    let fade_black_entity = commands.spawn_empty().id();
    let fade_white_entity = commands.spawn_empty().id();

    commands.entity(fade_black_entity).insert((
        FadeBlack,
        fade_black_name,
        Sprite::from_color(BLACK.with_alpha(0.0), vec2(WINDOW_WIDTH, WINDOW_HEIGHT)),
        FadeOpacity(0.0),
        AnimationPlayer::default(),
        AnimationGraphHandle(fade_black_graph_handle),
        AnimationTarget {
            id:     fade_black_target_id,
            player: fade_black_entity,
        },
        EffectsNodes {
            in_node:  fade_black_nodes[0],
            out_node: fade_black_nodes[1],
        },
        Transform::from_translation(Vec3::ZERO.with_z(Z_EFFECTS)),
        RENDER_LAYER_OVERLAY,
    ));
    commands.entity(fade_white_entity).insert((
        FadeWhite,
        fade_white_name,
        Sprite::from_color(WHITE.with_alpha(0.0), vec2(WINDOW_WIDTH, WINDOW_HEIGHT)),
        FadeOpacity(0.0),
        AnimationPlayer::default(),
        AnimationGraphHandle(fade_white_graph_handle),
        AnimationTarget {
            id:     fade_white_target_id,
            player: fade_white_entity,
        },
        EffectsNodes {
            in_node:  fade_white_nodes[0],
            out_node: fade_white_nodes[1],
        },
        Transform::from_translation(Vec3::ZERO.with_z(Z_EFFECTS)),
        RENDER_LAYER_OVERLAY,
    ));
}

fn update_fade(mut q_fade: Query<(&mut Sprite, &FadeOpacity), Changed<FadeOpacity>>) {
    q_fade
        .iter_mut()
        .for_each(|(mut sprite, FadeOpacity(alpha))| {
            sprite.color = sprite.color.with_alpha(*alpha);
        });
}

pub fn _fade_to_black(fade_black: Single<(&mut AnimationPlayer, &EffectsNodes), With<FadeBlack>>) {
    let (mut player, nodes) = fade_black.into_inner();
    player.stop_all().play(nodes.in_node);
}

pub fn _fade_from_black(fade_black: Single<(&mut AnimationPlayer, &EffectsNodes), With<FadeBlack>>) {
    let (mut player, nodes) = fade_black.into_inner();
    player.stop_all().play(nodes.out_node);
}

pub fn fade_to_white(fade_white: Single<(&mut AnimationPlayer, &EffectsNodes), With<FadeWhite>>) {
    let (mut player, nodes) = fade_white.into_inner();
    player.stop_all().play(nodes.in_node);
}

pub fn fade_from_white(fade_white: Single<(&mut AnimationPlayer, &EffectsNodes), With<FadeWhite>>) {
    let (mut player, nodes) = fade_white.into_inner();
    player.stop_all().play(nodes.out_node);
}

#[derive(Debug, Component)]
pub struct CinematicBars;

#[derive(Debug, Clone, Copy, Event)]
pub struct CinematicBarsIn;

#[derive(Debug, Clone, Copy, Event)]
pub struct CinematicBarsOut;

fn spawn_cinematic_bars(mut commands: Commands, asset_server: Res<AssetServer>) {
    const BAR_HEIGHT: f32 = WINDOW_HEIGHT / 8.0;
    let bar_sprite = Sprite::from_color(BLACK, vec2(WINDOW_WIDTH, BAR_HEIGHT));

    let bar_upper_name = Name::new("bar_upper");
    let bar_lower_name = Name::new("bar_lower");
    let bar_upper_target_id = AnimationTargetId::from_name(&bar_upper_name);
    let bar_lower_target_id = AnimationTargetId::from_name(&bar_lower_name);

    let bar_upper_in_y = (WINDOW_HEIGHT - BAR_HEIGHT) / 2.0;
    let bar_lower_in_y = -bar_upper_in_y;
    let bar_upper_out_y = bar_upper_in_y + BAR_HEIGHT;
    let bar_lower_out_y = bar_lower_in_y - BAR_HEIGHT;

    const DURATION: f32 = 2.0;
    let domain = interval(0.0, DURATION).unwrap();

    let (graph, nodes) = AnimationGraph::from_clips([
        asset_server.add({
            let mut clip_in = AnimationClip::default();
            clip_in.add_curve_to_target(
                bar_upper_target_id,
                AnimatableCurve::new(
                    animated_field!(Transform::translation),
                    EasingCurve::new(
                        vec3(0.0, bar_upper_out_y, Z_EFFECTS),
                        vec3(0.0, bar_upper_in_y, Z_EFFECTS),
                        EaseFunction::SmoothStep,
                    )
                    .reparametrize_linear(domain)
                    .unwrap(),
                ),
            );
            clip_in.add_curve_to_target(
                bar_lower_target_id,
                AnimatableCurve::new(
                    animated_field!(Transform::translation),
                    EasingCurve::new(
                        vec3(0.0, bar_lower_out_y, Z_EFFECTS),
                        vec3(0.0, bar_lower_in_y, Z_EFFECTS),
                        EaseFunction::SmoothStep,
                    )
                    .reparametrize_linear(domain)
                    .unwrap(),
                ),
            );
            clip_in.add_event_fn(DURATION, |commands, _entity, _time, _weight| {
                commands.send_event(CinematicBarsIn);
            });
            clip_in
        }),
        asset_server.add({
            let mut clip_out = AnimationClip::default();
            clip_out.add_curve_to_target(
                bar_upper_target_id,
                AnimatableCurve::new(
                    animated_field!(Transform::translation),
                    EasingCurve::new(
                        vec3(0.0, bar_upper_in_y, Z_EFFECTS),
                        vec3(0.0, bar_upper_out_y, Z_EFFECTS),
                        EaseFunction::SmoothStep,
                    )
                    .reparametrize_linear(domain)
                    .unwrap(),
                ),
            );
            clip_out.add_curve_to_target(
                bar_lower_target_id,
                AnimatableCurve::new(
                    animated_field!(Transform::translation),
                    EasingCurve::new(
                        vec3(0.0, bar_lower_in_y, Z_EFFECTS),
                        vec3(0.0, bar_lower_out_y, Z_EFFECTS),
                        EaseFunction::SmoothStep,
                    )
                    .reparametrize_linear(domain)
                    .unwrap(),
                ),
            );
            clip_out.add_event_fn(DURATION, |commands, _entity, _time, _weight| {
                commands.send_event(CinematicBarsOut);
            });
            clip_out
        }),
    ]);

    let root_entity = commands
        .spawn((
            CinematicBars,
            Name::new("Cinematic Bars"),
            AnimationPlayer::default(),
            AnimationGraphHandle(asset_server.add(graph)),
            EffectsNodes {
                in_node:  nodes[0],
                out_node: nodes[1],
            },
            Transform::default(),
            Visibility::default(),
            RENDER_LAYER_OVERLAY,
        ))
        .id();

    commands.spawn((
        bar_upper_name,
        ChildOf(root_entity),
        bar_sprite.clone(),
        AnimationTarget {
            id:     bar_upper_target_id,
            player: root_entity,
        },
        Transform::from_xyz(0.0, bar_upper_out_y, Z_EFFECTS),
        Visibility::default(),
        RENDER_LAYER_OVERLAY,
    ));

    commands.spawn((
        bar_lower_name,
        ChildOf(root_entity),
        bar_sprite,
        AnimationTarget {
            id:     bar_lower_target_id,
            player: root_entity,
        },
        Transform::from_xyz(0.0, bar_upper_out_y, Z_EFFECTS),
        Visibility::default(),
        RENDER_LAYER_OVERLAY,
    ));
}

pub fn cinematic_bars_in(
    cinematic_bars: Single<(&mut AnimationPlayer, &EffectsNodes), With<CinematicBars>>,
) {
    let (mut animation_player, nodes) = cinematic_bars.into_inner();
    animation_player.stop_all().play(nodes.in_node);
}

pub fn cinematic_bars_out(
    cinematic_bars: Single<(&mut AnimationPlayer, &EffectsNodes), With<CinematicBars>>,
) {
    let (mut player, nodes) = cinematic_bars.into_inner();
    player.stop_all().play(nodes.out_node);
}
