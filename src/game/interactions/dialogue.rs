use nohash_hasher::IsEnabled;
use std::hash::{Hash, Hasher};

use bevy::{
    animation::{AnimationTarget, AnimationTargetId, animated_field},
    asset::LoadState,
    // ecs::system::SystemId,
    prelude::*,
};
use bevy_text_animation::TextSimpleAnimator;
use serde::{Deserialize, Serialize};

use super::*;
use crate::{
    Blob, EnumMap, RENDER_LAYER_OVERLAY, WINDOW_HEIGHT, WINDOW_WIDTH, animation::SpriteAnimation,
    game::topdown::PlayerSpawnLocation,
};

#[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[source(InteractionState = InteractionState::Dialogue)]
enum DialogueState {
    #[default]
    Loading,
    Playing,
    Ending,
}

pub fn dialogue_plugin(app: &mut App) {
    app.add_systems(
        Startup,
        (spawn_cinematic_bars, add_new_dialogue, load_stored_dialogue),
    )
    .add_systems(
        Update,
        (
            add_stored_dialogue.run_if(resource_exists::<DialogueStored>),
            preload_dialogues.run_if(resource_exists_and_changed::<DialoguePreload>),
        ),
    )
    .add_systems(
        OnEnter(DialogueState::Loading),
        (fetch_dialouge, cinematic_bars_in),
    )
    .add_systems(
        Update,
        wait_for_loaded_and_bars.run_if(in_state(DialogueState::Loading)),
    )
    .add_systems(OnEnter(DialogueState::Playing), play_dialogue)
    .add_systems(
        Update,
        advance_dialogue
            .run_if(in_state(DialogueState::Playing).and(on_event::<InteractionAdvance>)),
    )
    .add_systems(
        OnEnter(DialogueState::Ending),
        (post_dialogue, cinematic_bars_out).chain(),
    )
    .add_systems(
        Update,
        conclude_dialogue.run_if(in_state(DialogueState::Ending).and(on_event::<CinematicBarsOut>)),
    )
    .add_event::<CinematicBarsIn>()
    .add_event::<CinematicBarsOut>()
    .add_sub_state::<DialogueState>()
    .init_resource::<DialoguePreload>()
    .init_resource::<DialogueStorage>()
    .register_type::<DialogueId>()
    .register_type::<DialogueElement>()
    .register_type::<DialogueLine>()
    .register_type::<DialogueAction>()
    .register_type::<ActionMode>()
    .register_type::<DialogueInfo>()
    .register_type::<DialogueStorage>()
    .register_type::<DialoguePreload>();

    #[cfg(not(target_arch = "wasm32"))]
    app.add_systems(
        Update,
        save_dialogue_storage_as_ron
            .run_if(|key_input: Res<ButtonInput<KeyCode>>| key_input.just_pressed(KeyCode::F12)),
    );
}

#[derive(Debug, Component)]
struct CinematicBars;

#[derive(Debug, Resource)]
struct CinematicBarsNodes {
    in_node:  AnimationNodeIndex,
    out_node: AnimationNodeIndex,
}

#[derive(Debug, Clone, Copy, Event)]
struct CinematicBarsIn;

#[derive(Debug, Clone, Copy, Event)]
struct CinematicBarsOut;

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

    const DURATION: f32 = 3.0;
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

    commands.insert_resource(CinematicBarsNodes {
        in_node:  nodes[0],
        out_node: nodes[1],
    });

    let root_entity = commands
        .spawn((
            CinematicBars,
            AnimationPlayer::default(),
            AnimationGraphHandle(asset_server.add(graph)),
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

#[derive(Debug, Component)]
struct DialogueCurrent;

fn fetch_dialouge(
    mut commands: Commands,
    current_id: Res<DialogueCurrentId>,
    q_dialogues: Query<(&DialogueId, Entity), With<DialogueRoot>>,
    storage: Res<DialogueStorage>,
    asset_server: Res<AssetServer>,
) {
    let current_dialouge_entity = q_dialogues
        .iter()
        .find_map(|(id, entity)| (*id == current_id.0).then_some(entity))
        .unwrap_or_else(|| load_dialogue(&mut commands, &storage, &asset_server, current_id.0));

    commands
        .entity(current_dialouge_entity)
        .insert(DialogueCurrent);
}

fn cinematic_bars_in(
    nodes: Res<CinematicBarsNodes>,
    mut cinematic_bars: Single<&mut AnimationPlayer, With<CinematicBars>>,
) {
    cinematic_bars.stop_all().play(nodes.in_node);
}

fn wait_for_loaded_and_bars(
    mut e_reader: EventReader<CinematicBarsIn>,
    mut dialogue_state: ResMut<NextState<DialogueState>>,
    mut bar_animation_done: Local<bool>,
    current_dialogue: Single<(&Dialogue, &mut Visibility), With<DialogueCurrent>>,
    asset_server: Res<AssetServer>,
) {
    if e_reader.read().count() > 0 {
        *bar_animation_done = true;
    }

    let (dialogue_info, mut dialogue_visibility) = current_dialogue.into_inner();
    if *bar_animation_done && dialogue_info.loaded(asset_server) {
        *bar_animation_done = false;
        *dialogue_visibility = Visibility::Visible;
        dialogue_state.set(DialogueState::Playing);
    }
}

fn play_dialogue(
    current_dialogues: Single<(&Dialogue, &mut AnimationPlayer), With<DialogueCurrent>>,
    interaction_prefix: Single<&mut TextSimpleAnimator, With<InteractionPrefix>>,
    interaction_text: Single<
        &mut TextSimpleAnimator,
        (With<InteractionText>, Without<InteractionPrefix>),
    >,
) {
    let (dialogue, mut element_animator) = current_dialogues.into_inner();

    element_animator.play(dialogue.nodes[0]);

    let TextAnimatorInfo { text, speed, delay } = &dialogue.texts[0];

    let speaker = dialogue.speakers[0];
    let mut prefix_animator = match speaker {
        Character::None => TextSimpleAnimator::default(),
        _ => TextSimpleAnimator::new(&format!("{speaker}: "), f32::MAX),
    };
    let mut text_animator = TextSimpleAnimator::new(text, *speed);

    if let Some(seconds) = delay {
        prefix_animator = prefix_animator.with_wait_before(*seconds);
        text_animator = text_animator.with_wait_before(*seconds);
    }

    *interaction_prefix.into_inner() = prefix_animator;
    *interaction_text.into_inner() = text_animator;
}

#[allow(clippy::type_complexity)]
fn advance_dialogue(
    current_dialogues: Single<(&Dialogue, &mut AnimationPlayer), With<DialogueCurrent>>,
    interaction_prefix: Single<&mut TextSimpleAnimator, With<InteractionPrefix>>,
    interaction_text: Single<
        (&mut Text, &mut TextSimpleAnimator),
        (With<InteractionText>, Without<InteractionPrefix>),
    >,
    mut dialogue_state: ResMut<NextState<DialogueState>>,
    mut line_index: Local<usize>,
) {
    *line_index += 1;

    let (dialogue, mut element_animator) = current_dialogues.into_inner();
    let (mut text, mut text_animator) = interaction_text.into_inner();

    if *line_index == dialogue.nodes.len() {
        // Dialogue is finished
        *line_index = 0;
        text.clear();
        dialogue_state.set(DialogueState::Ending);
        return;
    }

    if !element_animator.all_finished() {
        element_animator.adjust_speeds(256.0);
        return;
    }

    // info!("Playing animations index: {}", *line_index);
    element_animator
        .stop_all()
        .play(dialogue.nodes[*line_index]);

    let TextAnimatorInfo { text, speed, delay } = &dialogue.texts[*line_index];

    let speaker = dialogue.speakers[*line_index];
    let mut prefix_animator = match speaker {
        Character::None => TextSimpleAnimator::default(),
        _ => TextSimpleAnimator::new(&format!("{speaker}: "), f32::MAX),
    };
    let mut new_text_animator = TextSimpleAnimator::new(text, *speed);

    if let Some(seconds) = delay {
        prefix_animator = prefix_animator.with_wait_before(*seconds);
        new_text_animator = new_text_animator.with_wait_before(*seconds);
    }

    *interaction_prefix.into_inner() = prefix_animator;
    *text_animator = new_text_animator;
}

fn post_dialogue(mut commands: Commands, current_dialogue: Res<DialogueCurrentId>) {
    match current_dialogue.0 {
        DialogueId::UckoIntro => {
            commands.set_state(GameState::Bones);
            commands.insert_resource(PlayerSpawnLocation(vec3(1280.0, 128.0, 1.0)));
        }
        DialogueId::WizuckoWin => {
            commands.set_state(DialogueState::Loading);
            commands.insert_resource(DialogueCurrentId(DialogueId::WizuckoIntro));
        }
        _ => {}
    }
}

fn cinematic_bars_out(
    nodes: Res<CinematicBarsNodes>,
    mut cinematic_bars: Single<&mut AnimationPlayer, With<CinematicBars>>,
) {
    cinematic_bars.stop_all().play(nodes.out_node);
}

fn conclude_dialogue(
    mut commands: Commands,
    current_dialogue: Single<Entity, With<DialogueCurrent>>,
    interaction_panel: Single<Entity, With<InteractionPanel>>,
) {
    commands.entity(current_dialogue.into_inner()).despawn();
    commands.entity(interaction_panel.into_inner()).despawn();
    commands.set_state(InteractionState::None);
    commands.set_state(MovementState::Enabled);
    commands.remove_resource::<DialogueCurrentId>();
}

#[derive(
    Debug, Default, Clone, Copy, Component, PartialEq, Eq, Reflect, Serialize, Deserialize,
)]
#[reflect(Default, Component, Serialize, Deserialize)]
pub enum DialogueId {
    #[default]
    None,
    UckoIntro,
    NinjuckoIntro,
    WizuckoWin,
    WizuckoIntro,
}

impl Hash for DialogueId {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        hasher.write_usize(*self as usize);
    }
}

impl IsEnabled for DialogueId {}

#[derive(Debug, Component)]
struct DialogueElementMarker;

#[derive(Debug, Reflect)]
// A Sprite to be loaded for a Dialogue scene that can have its own animation
// and be animated via DialogueActions
struct DialogueElement {
    path: String,

    position:    Vec2,
    custom_size: Option<Vec2>,

    frames:  usize,
    fps:     u8,
    looping: bool,
}

impl DialogueElement {
    const DEFAULT_FPS: u8 = 12;

    fn new(path: &'static str) -> Self {
        DialogueElement {
            path:        path.to_string(),
            position:    Vec2::ZERO,
            custom_size: None,
            frames:      1,
            fps:         Self::DEFAULT_FPS,
            looping:     false,
        }
    }

    fn _position(mut self, position: Vec2) -> Self {
        self.position = position;
        self
    }

    fn custom_size(mut self, custom_size: Vec2) -> Self {
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

#[derive(Debug, Reflect)]
// Contains any actions for DialogueElements to be played during the line
struct DialogueLine {
    speaker: Character,
    text:    String,
    speed:   f32,
    delay:   Option<f32>,
    actions: Vec<DialogueAction>,
}

impl DialogueLine {
    const DEFAULT_TEXT_SPEED: f32 = 16.0; // chars per second

    fn new(speaker: Character, text: &'static str) -> Self {
        let text = text.to_string();
        DialogueLine {
            speaker,
            text,
            speed: Self::DEFAULT_TEXT_SPEED,
            delay: None,
            actions: vec![],
        }
    }

    fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    fn delay(mut self, delay: f32) -> Self {
        self.delay = Some(delay);
        self
    }

    fn add_action(mut self, action: DialogueAction) -> Self {
        self.actions.push(action);
        self
    }
}

#[derive(Debug, Reflect)]
enum ActionMode {
    Translate,
    _Rotate,
    Scale,
    Activate,
    Deactivate,
}

// A Transform animation to be played on an indexed DialogueElement
#[derive(Debug, Reflect)]
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

    fn _teleport(element: usize, start: Vec2, end: Vec2) -> Self {
        // this is jank af
        Self::new(element).start(start).end(end).duration(0.001)
    }

    fn activate(element: usize) -> Self {
        Self::new(element).mode(ActionMode::Activate)
    }

    fn deactivate(element: usize) -> Self {
        Self::new(element).mode(ActionMode::Deactivate)
    }

    fn mode(mut self, mode: ActionMode) -> Self {
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

    fn ease(mut self, ease: EaseFunction) -> Self {
        self.ease = ease;
        self
    }

    fn delay(mut self, delay: f32) -> Self {
        self.delay = delay;
        self
    }

    fn duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }
}

#[derive(Debug, Component)]
struct DialogueRoot;

// Information to construct a cutscene of dialogue with elements
// and lines with actions that act on those elements
#[derive(Debug, Reflect)]
struct DialogueInfo {
    elements: Vec<DialogueElement>,
    lines:    Vec<DialogueLine>,
}

#[derive(Debug, Component)]
struct Dialogue {
    elements: Vec<Handle<Image>>,

    // Should all have lengths equal to the number of lines
    speakers: Vec<Character>,
    texts:    Vec<TextAnimatorInfo>,
    nodes:    Vec<AnimationNodeIndex>,
}

impl Dialogue {
    fn loaded(&self, asset_server: Res<'_, AssetServer>) -> bool {
        self.elements.iter().all(|handle| {
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

#[derive(Debug, Clone)]
struct TextAnimatorInfo {
    text:  String,
    speed: f32,
    delay: Option<f32>,
}

impl TextAnimatorInfo {
    fn new(text: String, speed: f32, delay: Option<f32>) -> Self {
        TextAnimatorInfo { text, speed, delay }
    }
}

#[derive(Clone, Copy, Debug, Event)]
struct ElementActivated;

#[derive(Clone, Copy, Debug, Event)]
struct ElementDeactivated;

const ELEMENT_TILE_SIZE: UVec2 = UVec2::splat(64);

fn preload_dialogues(
    mut commands: Commands,
    mut preload: ResMut<DialoguePreload>,
    storage: Res<DialogueStorage>,
    asset_server: Res<AssetServer>,
) {
    preload.drain(..).for_each(|id| {
        load_dialogue(&mut commands, &storage, &asset_server, id);
    })
}

fn load_dialogue(
    commands: &mut Commands,
    storage: &Res<'_, DialogueStorage>,
    asset_server: &Res<'_, AssetServer>,
    id: DialogueId,
) -> Entity {
    let dialogue = storage
        .get(&id)
        .expect("Dialogue storage should have entries for every Id");

    info!("Loading Dialogue: {:?}", id);

    let root_entity = commands
        .spawn((
            DialogueRoot,
            id,
            AnimationPlayer::default(),
            Transform::default(),
            Visibility::default(),
            RENDER_LAYER_OVERLAY,
        ))
        .id();

    let mut activation_observer = Observer::new(activate_element);
    let mut deactivation_observer = Observer::new(deactivate_element);

    type ElementsInfo = (Vec<Handle<Image>>, Vec<AnimationTargetId>);
    let (image_handles, target_ids): ElementsInfo = dialogue
        .elements
        .iter()
        .map(|element| {
            let name = Name::new(element.path.clone());
            let target_id = AnimationTargetId::from_name(&name);

            let image_handle = asset_server.load(&element.path);
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

            let sprite_animation = {
                let sprite_animation =
                    SpriteAnimation::new(0, element.frames - 1, element.fps).paused();
                match element.looping {
                    true => sprite_animation.looping(),
                    false => sprite_animation,
                }
            };

            let element_entity = commands
                .spawn((
                    DialogueElementMarker,
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
                    Transform::default(),
                    Visibility::Hidden,
                    RENDER_LAYER_OVERLAY,
                ))
                .id();

            activation_observer.watch_entity(element_entity);
            deactivation_observer.watch_entity(element_entity);

            (image_handle, target_id)
        })
        .unzip();

    commands.spawn(activation_observer);
    commands.spawn(deactivation_observer);

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
                TextAnimatorInfo::new(line.text.clone(), line.speed, line.delay),
                asset_server.add(clip),
            )
        })
        .unzip();

    let speakers: Vec<Character> = dialogue.lines.iter().map(|line| line.speaker).collect();

    let (animation_graph, animation_nodes) = AnimationGraph::from_clips(clips);
    let animation_graph_handle: Handle<AnimationGraph> = asset_server.add(animation_graph);

    commands.entity(root_entity).insert((
        AnimationGraphHandle(animation_graph_handle),
        Dialogue {
            elements: image_handles,
            speakers,
            texts: text_animator_info,
            nodes: animation_nodes,
        },
    ));

    root_entity
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
        ActionMode::Scale => clip.add_curve_to_target(
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
        ActionMode::Activate => {
            clip.add_event_to_target(element_target_id, action.delay, ElementActivated)
        }
        ActionMode::Deactivate => {
            clip.add_event_to_target(element_target_id, action.delay, ElementDeactivated)
        }
    };

    clip
}

fn activate_element(
    trigger: Trigger<ElementActivated>,
    mut q_elements: Query<(&mut SpriteAnimation, &mut Visibility), With<DialogueElementMarker>>,
) {
    let (mut animation, mut visibility) = q_elements
        .get_mut(trigger.target())
        .expect("All Dialogue elements should have Sprite Animation and Visibility");
    animation.as_mut().play();
    *visibility = Visibility::Visible;
}

fn deactivate_element(
    trigger: Trigger<ElementDeactivated>,
    mut q_elements: Query<(&mut SpriteAnimation, &mut Visibility), With<DialogueElementMarker>>,
) {
    let (mut animation, mut visibility) = q_elements
        .get_mut(trigger.target())
        .expect("All Dialogue elements should have Sprite Animation and Visibility");
    animation.as_mut().pause();
    *visibility = Visibility::Hidden;
}

#[derive(Debug, Default, Deref, DerefMut, Resource, Reflect)]
#[reflect(Resource)]
struct DialogueStorage(EnumMap<DialogueId, DialogueInfo>);

#[derive(Debug, Resource)]
struct DialogueStored(Handle<Blob>);

fn load_stored_dialogue(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load("dialogue.ron");
    commands.insert_resource(DialogueStored(handle));
}

fn add_stored_dialogue(
    dialogue_stored: Res<DialogueStored>,
    assets_blob: Res<Assets<Blob>>,
    type_registry: Res<AppTypeRegistry>,
    mut dialogue_storage: ResMut<DialogueStorage>,
    mut commands: Commands,
    mut finished: Local<bool>,
) {
    // Just in case system runs again before ['DialogueStored'] is removed
    if *finished {
        return;
    }

    let Some(blob) = assets_blob.get(&dialogue_stored.0) else {
        info_once!("Stored dialogue still loading");
        return;
    };

    use bevy::{reflect::serde::ReflectDeserializer, scene::ron::Deserializer};
    use serde::de::DeserializeSeed;

    let type_registry = type_registry.read();

    let reflect_deserializer = ReflectDeserializer::new(&type_registry);
    let mut deserializer = Deserializer::from_bytes(&blob.bytes).unwrap();
    let reflect_value = reflect_deserializer.deserialize(&mut deserializer).unwrap();

    dialogue_storage.apply(&*reflect_value);
    commands.remove_resource::<DialogueStored>();
    *finished = true;
}

fn add_new_dialogue(mut dialogue_storage: ResMut<DialogueStorage>) {
    const SCENE_AREA_HEIGHT: f32 = 540.0;
    const _OFFSCREEN: Vec2 = Vec2::splat(-2048.0);

    dialogue_storage.insert(
        DialogueId::UckoIntro,
        DialogueInfo {
            elements: vec![
                DialogueElement::new("sprites/bucko/intro.png")
                    .custom_size(Vec2::splat(SCENE_AREA_HEIGHT))
                    .frames(16)
                    .fps(16)
                    .looping(),
                DialogueElement::new("sprites/bucko/grow.png")
                    .custom_size(Vec2::splat(SCENE_AREA_HEIGHT))
                    .frames(18)
                    .fps(16),
                DialogueElement::new("sprites/ucko/group.png"),
                DialogueElement::new("sprites/bucko/bones_1.png"),
                DialogueElement::new("sprites/bucko/bones_2.png"),
                DialogueElement::new("sprites/bucko/escape.png"),
            ],
            lines:    vec![
                DialogueLine::new(
                    Character::Bucko,
                    "Pardon me. Did I miss something? What's going on?",
                )
                .delay(7.5)
                .add_action(DialogueAction::activate(0))
                .add_action(
                    DialogueAction::new(0)
                        .mode(ActionMode::Scale)
                        .start(vec2(0.1, 0.1))
                        .end(vec2(1.0, 1.0))
                        .ease(EaseFunction::Steps(3, JumpAt::End))
                        .duration(6.0),
                )
                .add_action(DialogueAction::deactivate(0).delay(6.5))
                .add_action(DialogueAction::activate(1).delay(6.5)),
                DialogueLine::new(Character::Unknown, "...").speed(1.0),
                DialogueLine::new(Character::Bucko, "Uh oh..."),
                DialogueLine::new(Character::Bucko, "AAAAAAIIIIEEEEEE!!"),
            ],
        },
    );

    dialogue_storage.insert(
        DialogueId::NinjuckoIntro,
        DialogueInfo {
            elements: vec![
                DialogueElement::new("sprites/ninjucko/idle.png")
                    .frames(4)
                    .fps(8)
                    .looping(),
            ],
            lines:    vec![],
        },
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn save_dialogue_storage_as_ron(
    type_registry: Res<AppTypeRegistry>,
    dialogue_storage: Res<DialogueStorage>,
) {
    static NEW_DIALOGUE_RON_PATH: &str = "assets/dialogue_new.ron";

    use bevy::{
        reflect::serde::ReflectSerializer,
        scene::ron::ser::{PrettyConfig, to_string_pretty},
        tasks::IoTaskPool,
    };

    let type_registry = type_registry.read();
    let serializer = ReflectSerializer::new(dialogue_storage.as_ref(), &type_registry);
    let ron_string = to_string_pretty(&serializer, PrettyConfig::default()).unwrap();

    IoTaskPool::get()
        .spawn(async move {
            std::fs::File::create(NEW_DIALOGUE_RON_PATH)
                .and_then(|mut file| std::io::Write::write(&mut file, ron_string.as_bytes()))
                .expect("Error while writing scene to file");
        })
        .detach();
}

// #[derive(Debug, Deref, Resource)]
// struct DialogueFinishedSystems(BuckoNoHashHashmap<DialogueId, SystemId>);

// impl FromWorld for DialogueFinishedSystems {
//     fn from_world(world: &mut World) -> Self {
//         let mut systems: BuckoNoHashHashmap<DialogueId, SystemId> =
//             HashMap::with_hasher(BuildBuckoNoHashHasher::default());

//         systems.insert(
//             DialogueId::UckoIntro,
//             world.register_system(|mut commands: Commands| {
//                 commands.set_state(GameState::Loading);
//             }),
//         );

//         DialogueFinishedSystems(systems)
//     }
// }
