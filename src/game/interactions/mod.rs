use std::sync::Arc;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::*;
use dialogue::*;

pub mod dialogue;

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum InteractionState {
    #[default]
    None,
    Text,
    Monologue,
    Dialogue,
}

pub fn interactions_plugin(app: &mut App) {
    app.add_plugins(dialogue_plugin)
        .add_systems(
            Update,
            (
                advance_interaction_text
                    .run_if(pressed_advance_key.and(any_with_component::<InteractionText>)),
                conclude_text_interaction
                    .run_if(in_state(InteractionState::Text).and(on_event::<InteractionAdvance>)),
            ),
        )
        .add_event::<InteractionAdvance>()
        .init_state::<InteractionState>()
        .register_type::<EntityInteraction>()
        .register_type::<InteractTarget>();
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
}

#[derive(Debug, Clone, Component, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub enum EntityInteraction {
    Text(String),
    Monologue,
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
    special_interactions: Query<&SpecialInteraction, With<EntityInteraction>>,
    mut commands: Commands,
) {
    const CLEAR: f32 = 0.0;
    const OPACITY_75: f32 = 0.75;

    let Some(interaction) = input else {
        debug!("Entity Interaction input invalid");
        return;
    };

    commands.set_state(MovementState::Disabled);
    match interaction {
        EntityInteraction::Text(text) => {
            commands.set_state(InteractionState::Text);
            commands
                .spawn(interaction_panel(OPACITY_75))
                .with_child(interaction_text(&text));
        }
        EntityInteraction::Monologue => {
            commands.set_state(InteractionState::Monologue);
        }
        EntityInteraction::Dialogue(id) => {
            if id == DialogueId::None {
                commands.set_state(MovementState::Enabled);
                return;
            }

            commands.set_state(InteractionState::Dialogue);
            commands.insert_resource(DialogueCurrentId(id));
            commands
                .spawn(interaction_panel(CLEAR))
                .with_child(interaction_text(""));
        }
        EntityInteraction::Special(entity) => {
            special_interactions
                .get(entity)
                .expect("Entity should have SpecialInteraction component")
                .0(&mut commands, entity);
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
    commands.set_state(MovementState::Enabled);
}

#[derive(Debug, Component)]
struct InteractionPanel;

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
