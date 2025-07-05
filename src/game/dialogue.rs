use bevy::{
    animation::{AnimationTarget, AnimationTargetId, animated_field},
    asset::LoadState,
    prelude::*,
};
use bevy_text_animation::TextSimpleAnimator;
use serde::{Deserialize, Serialize};

use super::*;
use crate::{RENDER_LAYER_OVERLAY, animation::SpriteAnimation};

// #[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
// #[source(InteractionState = InteractionState::Dialogue)]
// enum DialogueState {
//     #[default]
//     Loading,
//     Playing,
//     Ending,
// }

pub fn dialogue_plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            initialize_dialogues.run_if(resource_exists_and_changed::<DialoguePreload>),
            play_and_advance_dialogue.run_if(
                resource_added::<DialogueCurrentId>
                    .or(in_state(InteractionState::Dialogue).and(on_event::<InteractionAdvance>)),
            ),
        ),
    )
    .init_resource::<DialogueStorage>()
    .init_resource::<DialoguePreload>()
    .register_type::<DialoguePreload>()
    .register_type::<DialogueId>();
}

fn play_and_advance_dialogue(
    mut commands: Commands,
    current_id: Res<DialogueCurrentId>,
    asset_server: Res<AssetServer>,
    mut q_dialogues: Query<
        (
            Entity,
            &DialogueId,
            &DialogueInfo,
            &mut AnimationPlayer,
            &mut Visibility,
        ),
        With<DialogueRoot>,
    >,
    interaction_text: Single<&mut TextSimpleAnimator, With<InteractionText>>,
    interaction_panel: Single<Entity, With<InteractionPanel>>,
    mut line_index: Local<usize>,
) {
    let (dialogue_entity, _id, info, mut animation_player, mut visibility) = q_dialogues
        .iter_mut()
        .find(|(_entity, id, ..)| &current_id.0 == *id)
        .expect("Current Dialouge should be spawned in");

    if !info.loaded(asset_server) {
        info_once!("Dialogue assets still loading");
        return;
    }

    if *line_index == info.nodes.len() {
        // info!("Dialogue is finished");
        *line_index = 0;
        commands.entity(dialogue_entity).despawn();
        commands.entity(interaction_panel.into_inner()).despawn();
        commands.set_state(InteractionState::None);
        commands.set_state(MovementState::Enabled);
        commands.remove_resource::<DialogueCurrentId>();
        return;
    }

    // info!("Playing animations index: {}", *line_index);

    *visibility = Visibility::Visible;
    animation_player.stop_all().play(info.nodes[*line_index]);
    let TextAnimatorInfo { text, speed } = info.texts[*line_index];
    *interaction_text.into_inner() = TextSimpleAnimator::new(text, speed);

    *line_index += 1;
    // info!("Line index incremented: {}", *line_index);
}

#[derive(Debug, Default, Clone, Copy, Component, PartialEq, Reflect, Serialize, Deserialize)]
#[reflect(Default, Component, Serialize, Deserialize)]
pub enum DialogueId {
    #[default]
    None,
    UckoIntro,
    NinjuckoIntro,
    WizuckoWin,
    WizuckoIntro,
}

#[derive(Debug)]
// A Sprite to be loaded for a Dialogue scene that can have its own animation
// and be animated via DialogueActions
struct DialogueElement {
    path: &'static str,

    custom_size: Option<Vec2>,

    frames:  usize,
    fps:     u8,
    looping: bool,
}

impl DialogueElement {
    const DEFAULT_FPS: u8 = 12;

    fn new(path: &'static str) -> Self {
        DialogueElement {
            path,
            custom_size: None,
            frames: 1,
            fps: Self::DEFAULT_FPS,
            looping: false,
        }
    }

    fn _custom_size(mut self, custom_size: Vec2) -> Self {
        self.custom_size = Some(custom_size);
        self
    }

    fn fps(mut self, fps: u8) -> Self {
        self.fps = fps;
        self
    }

    fn frames(mut self, frames: usize) -> Self {
        self.frames = frames;
        self
    }

    fn looping(mut self) -> Self {
        self.looping = true;
        self
    }
}

#[derive(Debug)]
// Contains any actions for DialogueElements to be played during the line
struct DialogueLine {
    text:    &'static str,
    speed:   f32,
    actions: Vec<DialogueAction>,
}

impl DialogueLine {
    fn new(line: &'static str) -> Self {
        DialogueLine {
            text:    line,
            speed:   16.0,
            actions: vec![],
        }
    }

    fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    fn add_action(mut self, action: DialogueAction) -> Self {
        self.actions.push(action);
        self
    }
}

#[derive(Debug)]
enum ActionMode {
    Translate,
    _Rotate,
    _Scale,
}

// A Transform animation to be played on an indexed DialogueElement
#[derive(Debug)]
struct DialogueAction {
    element: usize,

    mode:  ActionMode,
    start: Vec2,
    end:   Vec2,
    ease:  EaseFunction,

    delay:    f32,
    duration: f32,
}

impl DialogueAction {
    fn new(element: usize) -> Self {
        DialogueAction {
            element,
            mode: ActionMode::Translate,
            start: Vec2::default(),
            end: Vec2::default(),
            ease: EaseFunction::Linear,
            delay: 0.0,
            duration: 1.0,
        }
    }

    fn _mode(mut self, mode: ActionMode) -> Self {
        self.mode = mode;
        self
    }

    fn start(mut self, start: Vec2) -> Self {
        self.start = start;
        self
    }

    fn end(mut self, end: Vec2) -> Self {
        self.end = end;
        self
    }

    fn _ease(mut self, ease: EaseFunction) -> Self {
        self.ease = ease;
        self
    }

    fn _delay(mut self, delay: f32) -> Self {
        self.delay = delay;
        self
    }

    fn _duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }
}

#[derive(Debug, Component)]
struct DialogueRoot;

// A cutscene of dialogue with elements and lines with actions that act on those elements
#[derive(Debug)]
struct Dialogue {
    _id:      DialogueId,
    elements: Vec<DialogueElement>,

    lines: Vec<DialogueLine>,
}
#[derive(Debug, Component)]
struct DialogueInfo {
    images: Vec<Handle<Image>>,
    texts:  Vec<TextAnimatorInfo>,
    nodes:  Vec<AnimationNodeIndex>,
}

impl DialogueInfo {
    fn loaded(&self, asset_server: Res<'_, AssetServer>) -> bool {
        self.images.iter().all(|handle| {
            matches!(
                asset_server.get_load_state(handle.id()),
                Some(LoadState::Loaded)
            )
        })
    }
}

#[derive(Debug, Deref, Resource)]
pub struct DialogueCurrentId(pub DialogueId);

#[derive(Debug, Default, Deref, DerefMut, Resource, Reflect)]
#[reflect(Debug, Resource)]
pub struct DialoguePreload(Vec<DialogueId>);

#[derive(Debug, Clone, Copy)]
struct TextAnimatorInfo {
    text:  &'static str,
    speed: f32,
}

impl TextAnimatorInfo {
    fn new(text: &'static str, speed: f32) -> Self {
        TextAnimatorInfo { text, speed }
    }
}

#[derive(Clone, Copy, Debug, Event)]
struct ClipStarted;

const ELEMENT_TILE_SIZE: UVec2 = UVec2::splat(64);

fn initialize_dialogues(
    mut commands: Commands,
    preload: Res<DialoguePreload>,
    storage: Res<DialogueStorage>,
    asset_server: Res<AssetServer>,
) {
    preload.iter().for_each(|&id| {
        let dialogue = &storage[id as usize];

        let root_entity = commands
            .spawn((
                DialogueRoot,
                id,
                AnimationPlayer::default(),
                Transform::default(),
                Visibility::Hidden,
                RENDER_LAYER_OVERLAY,
            ))
            .id();

        type HandlesAndIds = (Vec<Handle<Image>>, Vec<AnimationTargetId>);
        let (image_handles, target_ids): HandlesAndIds = dialogue
            .elements
            .iter()
            .map(|element| {
                let name = Name::new(element.path);
                let target_id = AnimationTargetId::from_name(&name);

                let image_handle = asset_server.load(element.path);
                let texture_atlas = TextureAtlas {
                    layout: asset_server.add(TextureAtlasLayout::from_grid(
                        ELEMENT_TILE_SIZE,
                        element.frames as u32,
                        1,
                        None,
                        None,
                    )),
                    index:  0,
                };

                let mut sprite_animation =
                    SpriteAnimation::new(0, element.frames - 1, element.fps).paused();
                if element.looping {
                    sprite_animation = sprite_animation.looping();
                }

                commands
                    .spawn((
                        name,
                        AnimationTarget {
                            player: root_entity,
                            id:     target_id,
                        },
                        ChildOf(root_entity),
                        Sprite {
                            image: image_handle.clone_weak(),
                            texture_atlas: Some(texture_atlas),
                            custom_size: element.custom_size,
                            ..Default::default()
                        },
                        sprite_animation,
                        Transform::from_xyz(-2000.0, -2000.0, 0.0),
                        Visibility::Inherited,
                        RENDER_LAYER_OVERLAY,
                    ))
                    .observe(play_sprite_animation);

                (image_handle, target_id)
            })
            .unzip();

        type AnimatorInfo = (Vec<TextAnimatorInfo>, Vec<Handle<AnimationClip>>);
        let (text_animator_info, clips): AnimatorInfo = dialogue
            .lines
            .iter()
            .map(|line| {
                let clip = line
                    .actions
                    .iter()
                    .fold(AnimationClip::default(), |clip, action| {
                        add_action_to_clip(&target_ids, clip, action)
                    });

                (
                    TextAnimatorInfo::new(line.text, line.speed),
                    asset_server.add(clip),
                )
            })
            .unzip();

        let (animation_graph, animation_nodes) = AnimationGraph::from_clips(clips);
        let animation_graph_handle: Handle<AnimationGraph> = asset_server.add(animation_graph);

        commands.entity(root_entity).insert((
            AnimationGraphHandle(animation_graph_handle),
            DialogueInfo {
                images: image_handles,
                texts:  text_animator_info,
                nodes:  animation_nodes,
            },
        ));
    });
}

fn play_sprite_animation(
    trigger: Trigger<ClipStarted>,
    mut q_sprite_animations: Query<&mut SpriteAnimation, With<AnimationTarget>>,
) {
    q_sprite_animations
        .get_mut(trigger.target())
        .expect("Trigger target should have Sprite Animation")
        .play();
}

fn add_action_to_clip(
    target_ids: &[AnimationTargetId],
    mut clip: AnimationClip,
    action: &DialogueAction,
) -> AnimationClip {
    let z_offset = Z_SPRITES + (action.element as f32 * 0.01);
    let element_target_id = target_ids[action.element];
    let domain = interval(action.delay, action.delay + action.duration).unwrap();

    match action.mode {
        ActionMode::Translate => clip.add_curve_to_target(
            element_target_id,
            AnimatableCurve::new(
                animated_field!(Transform::translation),
                EasingCurve::new(
                    action.start.extend(z_offset),
                    action.end.extend(z_offset),
                    action.ease,
                )
                .reparametrize_linear(domain)
                .unwrap(),
            ),
        ),
        ActionMode::_Rotate => clip.add_curve_to_target(
            element_target_id,
            AnimatableCurve::new(
                animated_field!(Transform::rotation),
                EasingCurve::new(
                    Quat::from_rotation_z(action.start.to_angle()),
                    Quat::from_rotation_z(action.end.to_angle()),
                    action.ease,
                )
                .reparametrize_linear(domain)
                .unwrap(),
            ),
        ),
        ActionMode::_Scale => clip.add_curve_to_target(
            element_target_id,
            AnimatableCurve::new(
                animated_field!(Transform::scale),
                EasingCurve::new(
                    action.start.extend(1.0),
                    action.end.extend(1.0),
                    action.ease,
                )
                .reparametrize_linear(domain)
                .unwrap(),
            ),
        ),
    };

    clip.add_event_to_target(element_target_id, 0.0, ClipStarted);

    clip
}

#[derive(Debug, Deref, Resource)]
struct DialogueStorage(Vec<Dialogue>);

impl FromWorld for DialogueStorage {
    fn from_world(_world: &mut World) -> Self {
        DialogueStorage(vec![
            Dialogue {
                _id:      DialogueId::None,
                elements: vec![],
                lines:    vec![],
            },
            Dialogue {
                _id:      DialogueId::UckoIntro,
                elements: vec![
                    DialogueElement::new("sprites/bucko/intro.png"),
                    DialogueElement::new("sprites/ucko/group.png"),
                    DialogueElement::new("sprites/bucko/bones_1.png"),
                    DialogueElement::new("sprites/bucko/bones_2.png"),
                    DialogueElement::new("sprites/bucko/escape.png"),
                ],
                lines:    vec![
                    DialogueLine::new("").add_action(
                        DialogueAction::new(0)
                            .start(vec2(0.0, 0.0))
                            .end(vec2(640.0, 0.0)),
                    ),
                    DialogueLine::new("Pardon me. Did I miss something? What's going on?"),
                    DialogueLine::new("...").speed(1.0),
                    DialogueLine::new("Uh oh..."),
                    DialogueLine::new("AAAAAAIIIIEEEEEE!!"),
                ],
            },
            Dialogue {
                _id:      DialogueId::NinjuckoIntro,
                elements: vec![
                    DialogueElement::new("sprites/ninjucko/idle.png")
                        .frames(4)
                        .fps(8)
                        .looping(),
                ],
                lines:    vec![],
            },
        ])
    }
}
