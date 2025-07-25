use std::sync::Arc;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::*;
use dialogue::*;
use monologue::*;

pub mod dialogue;
pub mod monologue;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, SubStates)]
#[source(InGame = InGame)]
pub enum InteractionState {
    #[default]
    None,
    Text,
    Dialogue,
}

pub fn interactions_plugin(app: &mut App) {
    app.add_plugins(dialogue_plugin)
        .add_systems(OnExit(InteractionState::None), disable_movement)
        .add_systems(OnExit(InteractionState::Text), enable_movement)
        .add_systems(OnExit(InteractionState::Dialogue), enable_movement)
        .add_systems(
            Update,
            (
                advance_interaction_text
                    .run_if(pressed_advance_key.and(any_with_component::<InteractionPanel>)),
                conclude_text_interaction
                    .run_if(in_state(InteractionState::Text).and(on_event::<InteractionAdvance>)),
            ),
        )
        .add_event::<InteractionAdvance>()
        .init_state::<InteractionState>()
        .init_resource::<MonologueServer>()
        .register_type::<EntityInteraction>()
        .register_type::<InteractTarget>()
        .register_type::<Monologue>()
        .register_type::<MonologueId>()
        .register_type::<MonologueServer>();
}

#[derive(Debug, Event)]
struct InteractionAdvance;

// Current entity with [EntityInteraction] in focus
#[derive(Debug, Default, Component, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct InteractTarget(Option<Entity>);

impl InteractTarget {
    pub fn set(&mut self, entity: Entity) {
        self.0 = Some(entity);
    }

    pub fn clear(&mut self) {
        self.0 = None;
    }

    pub fn get(&self) -> Option<&Entity> {
        self.as_ref()
    }
}

#[derive(Debug, Clone, Component, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub enum EntityInteraction {
    Text(String),
    Monologue(MonologueId),
    Dialogue(DialogueId),
    Special(Entity),
}

#[derive(Component)]
pub struct SpecialInteraction(SpecialInteractionFn);
type SpecialInteractionFn = Arc<dyn Fn(&mut Commands, Entity) + Send + Sync>;

impl core::fmt::Debug for SpecialInteraction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("SpecialInteraction").finish()
    }
}

impl SpecialInteraction {
    pub fn new(func: impl Fn(&mut Commands, Entity) + Send + Sync + 'static) -> Self {
        SpecialInteraction(Arc::new(func))
    }
}

pub fn play_interactions(
    In(input): In<Option<EntityInteraction>>,
    special_interactions: Query<&SpecialInteraction>,
    mut monologue_server: ResMut<MonologueServer>,
    mut commands: Commands,
) {
    const CLEAR: f32 = 0.0;
    const OPACITY_75: f32 = 0.75;

    let Some(interaction) = input else {
        debug!("Entity Interaction input invalid");
        return;
    };

    match interaction {
        EntityInteraction::Text(text) => {
            commands.set_state(InteractionState::Text);
            commands
                .spawn(interaction_panel(OPACITY_75))
                .with_child(interaction_text(&text));
        }
        EntityInteraction::Monologue(id) => {
            let text = monologue_server.next_line(&id);
            commands.set_state(InteractionState::Text);
            commands
                .spawn(interaction_panel(OPACITY_75))
                .with_child(interaction_text(text));
        }
        EntityInteraction::Dialogue(id) => {
            if id == DialogueId::None {
                return;
            }

            commands.set_state(InteractionState::Dialogue);
            commands.insert_resource(DialogueCurrentId(id));
            commands
                .spawn(interaction_panel(CLEAR))
                .with_child(interaction_prefix())
                .with_child(interaction_text(""));
        }
        EntityInteraction::Special(entity) => {
            let Ok(SpecialInteraction(func)) = special_interactions.get(entity) else {
                warn!("Entity {} has no SpecialInteraction", entity);
                return;
            };
            func(&mut commands, entity);
        }
    }
}

fn advance_interaction_text(
    mut e_writer: EventWriter<InteractionAdvance>,
    interaction_text: Single<(&mut TextSimpleAnimator, &mut Text), With<InteractionText>>,
) {
    // Skip playing text animation
    let (mut animator, mut text) = interaction_text.into_inner();
    if animator.is_playing() || animator.is_waiting() {
        text.0 = animator.text.clone();
        animator.stop();
        return;
    }

    e_writer.write(InteractionAdvance);
}

fn conclude_text_interaction(
    mut commands: Commands,
    interaction_panel: Single<Entity, With<InteractionPanel>>,
) {
    commands.entity(interaction_panel.into_inner()).despawn();
    commands.set_state(InteractionState::None);
}

#[derive(Debug, Component)]
struct InteractionPanel;

#[derive(Debug, Component)]
struct InteractionPrefix;

#[derive(Debug, Component)]
struct InteractionText;

fn interaction_panel(opacity: f32) -> impl Bundle {
    (
        InteractionPanel,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(87.5),
            left: Val::Percent(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(12.5),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::linear_rgba(0.0, 0.0, 0.0, opacity)),
    )
}

fn interaction_prefix() -> impl Bundle {
    (
        InteractionPrefix,
        Text::new(""),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(WHITE.into()),
        TextSimpleAnimator::default(),
    )
}

fn interaction_text(text: &str) -> impl Bundle {
    (
        InteractionText,
        Text::new(""),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(WHITE.into()),
        TextSimpleAnimator::new(text, 16.0),
    )
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Default, Reflect)]
pub enum Character {
    #[default]
    None,
    Unknown,
    Bucko,
    Ninjucko,
    Wizucko,
    Bartucko,
    Brock,
    Maducko,
    Cowbucko,
}

impl std::fmt::Display for Character {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Character::Unknown => write!(f, "???"),
            Character::Brock => write!(f, "Bane \"The Brock\" Bronson"),
            _ => write!(f, "{:?}", self),
        }
    }
}
