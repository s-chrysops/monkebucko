#![allow(clippy::type_complexity)]
use bevy::{color::palettes::css::*, ecs::spawn::SpawnWith, prelude::*};
use bevy_persistent::Persistent;

use crate::progress::{Progress, ProgressStorage, SaveSlot};

use super::{AppState, Settings, despawn_screen};

const TEXT_COLOR: Color = Color::Srgba(WHITE_SMOKE);

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const HOVERED_PRESSED_BUTTON: Color = Color::srgb(0.25, 0.65, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates)]
#[source(AppState = AppState::Menu)]
enum MenuState {
    #[default]
    Main,
    Settings,
    Data,
}

#[derive(Component)]
struct OnMainMenu;

#[derive(Component)]
struct OnSettings;

#[derive(Component)]
struct OnData;

#[derive(Component)]
struct SelectedOption;

#[derive(Component)]
enum MenuButtonAction {
    Play,
    Settings,
    SaveSettings,
    BackToMainMenu,
    Quit,
}

#[derive(Debug, Clone, Copy, Component, PartialEq)]
enum RadioSetting {
    Sound,
    Music,
}

#[derive(Component, Clone, Copy)]
struct RadioValue(u32);

pub fn menu_plugin(app: &mut App) {
    app.add_sub_state::<MenuState>()
        .add_systems(OnEnter(MenuState::Main), main_menu_setup)
        .add_systems(OnExit(MenuState::Main), despawn_screen::<OnMainMenu>)
        .add_systems(OnEnter(MenuState::Settings), settings_menu_setup)
        .add_systems(OnExit(MenuState::Settings), despawn_screen::<OnSettings>)
        .add_systems(OnEnter(MenuState::Data), setup_data_menu)
        .add_systems(OnExit(MenuState::Data), despawn_screen::<OnData>)
        .add_systems(
            Update,
            (menu_action, button_system).run_if(in_state(AppState::Menu)),
        )
        .add_systems(
            Update,
            radio_settings_system.run_if(in_state(MenuState::Settings)),
        )
        .add_systems(Update, start_game.run_if(in_state(MenuState::Data)));
}

fn menu_action(
    interaction_query: Query<
        (&Interaction, &MenuButtonAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut app_exit_events: EventWriter<AppExit>,
    mut menu_state: ResMut<NextState<MenuState>>,
    // mut game_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
) {
    for (interaction, menu_button_action) in &interaction_query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match menu_button_action {
            MenuButtonAction::Quit => {
                app_exit_events.write(AppExit::Success);
            }
            MenuButtonAction::Play => {
                menu_state.set(MenuState::Data);
            }
            MenuButtonAction::Settings => menu_state.set(MenuState::Settings),
            MenuButtonAction::SaveSettings => {
                commands.run_system_cached(save_settings);
                menu_state.set(MenuState::Main);
            }
            MenuButtonAction::BackToMainMenu => menu_state.set(MenuState::Main),
        }
    }
}

fn main_menu_setup(mut commands: Commands, _asset_server: Res<AssetServer>) {
    let button_node = Node {
        width: Val::Px(300.0),
        height: Val::Px(65.0),
        margin: UiRect::all(Val::Px(20.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };

    let button_text_font = TextFont {
        font_size: 32.0,
        ..default()
    };

    let root_node = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            OnMainMenu,
        ))
        .id();

    let title = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BLACK.into()),
            children![
                Text::new("monkebucko"),
                TextFont {
                    font_size: 64.0,
                    ..default()
                },
                TextColor(TEXT_COLOR),
            ],
        ))
        .id();

    let menu_buttons = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(SLATE_GREY.into()),
            children![
                // Display three buttons for each action available from the main menu:
                // - new game
                // - settings
                // - quit
                (
                    Button,
                    button_node.clone(),
                    BackgroundColor(NORMAL_BUTTON),
                    MenuButtonAction::Play,
                    children![
                        // (ImageNode::new(right_icon), button_icon_node.clone()),
                        (
                            Text::new("Play"),
                            button_text_font.clone(),
                            TextColor(TEXT_COLOR),
                        ),
                    ]
                ),
                (
                    Button,
                    button_node.clone(),
                    BackgroundColor(NORMAL_BUTTON),
                    MenuButtonAction::Settings,
                    children![
                        // (ImageNode::new(wrench_icon), button_icon_node.clone()),
                        (
                            Text::new("Settings"),
                            button_text_font.clone(),
                            TextColor(TEXT_COLOR),
                        ),
                    ]
                ),
                (
                    Button,
                    button_node,
                    BackgroundColor(NORMAL_BUTTON),
                    MenuButtonAction::Quit,
                    children![
                        // (ImageNode::new(exit_icon), button_icon_node),
                        (Text::new("Quit"), button_text_font, TextColor(TEXT_COLOR),),
                    ]
                ),
            ],
        ))
        .id();

    commands
        .entity(root_node)
        .add_children(&[title, menu_buttons]);
}

fn settings_menu_setup(
    mut commands: Commands,
    _asset_server: Res<AssetServer>,
    settings: Res<Persistent<Settings>>,
) {
    let root_node = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            OnSettings,
        ))
        .id();

    let Settings {
        up: _up,
        down: _down,
        left: _left,
        right: _right,
        jump: _jump,
        swap: _swap,
        interact: _interact,

        sound_vol,
        music_vol,
    } = *settings.get();

    let settings_node = Node {
        width: Val::Percent(80.0),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    };

    let button_node = Node {
        width: Val::Px(300.0),
        height: Val::Px(65.0),
        margin: UiRect::all(Val::Px(20.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };

    let button_text_font = TextFont {
        font_size: 32.0,
        ..default()
    };

    let key_bindings_node = commands
        .spawn((
            Node {
                width: Val::Percent(80.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(SLATE_GREY.into()),
            children![(
                Text::new("Key Bindings"),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(TEXT_COLOR),
            )],
        ))
        .id();

    let music_volume_node = commands
        .spawn((
            settings_node.clone(),
            BackgroundColor(DARK_GREY.into()),
            Children::spawn((
                Spawn((
                    Text::new("Music Volume"),
                    TextFont {
                        font_size: 32.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                )),
                SpawnWith(move |parent: &mut ChildSpawner| {
                    (0..=10).for_each(|level| {
                        let mut entity = parent.spawn((
                            RadioSetting::Music,
                            Button,
                            Node {
                                width: Val::Px(32.0),
                                height: Val::Px(48.0),
                                margin: UiRect::all(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(NORMAL_BUTTON),
                            RadioValue(level),
                        ));

                        let volume = 0.1 * level as f32;
                        if music_vol == volume {
                            entity.insert(SelectedOption);
                        }
                    });
                }),
            )),
        ))
        .id();

    let sound_volume_node = commands
        .spawn((
            settings_node.clone(),
            BackgroundColor(DARK_GREY.into()),
            Children::spawn((
                Spawn((
                    Text::new("Sound Volume"),
                    TextFont {
                        font_size: 32.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                )),
                SpawnWith(move |parent: &mut ChildSpawner| {
                    (0..=10).for_each(|level| {
                        let mut entity = parent.spawn((
                            RadioSetting::Sound,
                            Button,
                            Node {
                                width: Val::Px(32.0),
                                height: Val::Px(48.0),
                                margin: UiRect::all(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(NORMAL_BUTTON),
                            RadioValue(level),
                        ));

                        let volume = 0.1 * level as f32;
                        if sound_vol == volume {
                            entity.insert(SelectedOption);
                        }
                    });
                }),
            )),
        ))
        .id();

    let navigation_node = commands
        .spawn((
            settings_node.clone(),
            BackgroundColor(SLATE_GRAY.into()),
            children![
                (
                    Button,
                    button_node.clone(),
                    BackgroundColor(NORMAL_BUTTON),
                    MenuButtonAction::BackToMainMenu,
                    children![
                        // (ImageNode::new(right_icon), button_icon_node.clone()),
                        (
                            Text::new("Back"),
                            button_text_font.clone(),
                            TextColor(TEXT_COLOR),
                        ),
                    ]
                ),
                (
                    Button,
                    button_node.clone(),
                    BackgroundColor(NORMAL_BUTTON),
                    MenuButtonAction::SaveSettings,
                    children![
                        // (ImageNode::new(wrench_icon), button_icon_node.clone()),
                        (
                            Text::new("Save & Exit"),
                            button_text_font.clone(),
                            TextColor(TEXT_COLOR),
                        ),
                    ]
                ),
            ],
        ))
        .id();

    commands.entity(root_node).add_children(&[
        key_bindings_node,
        music_volume_node,
        sound_volume_node,
        navigation_node,
    ]);
}

#[derive(Debug, Component)]
struct TimePlayed;

#[derive(Debug, Component)]
struct StartButton;

fn setup_data_menu(mut commands: Commands, progress_storage: Res<Persistent<ProgressStorage>>) {
    let button_text_font = TextFont {
        font_size: 32.0,
        ..default()
    };

    let root_node = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            OnData,
        ))
        .id();

    // Slot Selection
    let slot_selection = commands
        .spawn((
            ChildOf(root_node),
            Node {
                width: Val::Percent(50.0),
                height: Val::Percent(70.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .id();

    let slots = [SaveSlot::SlotA, SaveSlot::SlotB, SaveSlot::SlotC];
    progress_storage
        .iter()
        .zip(slots)
        .for_each(|(progress, slot)| {
            let time_played = progress
                .as_ref()
                .map(|progress| {
                    let seconds = progress.time_played.as_secs();
                    let minutes = seconds / 60;
                    let hours = minutes / 60;
                    format!("{:02}:{:02}:{:02}", hours, minutes % 60, seconds % 3600)
                })
                .unwrap_or("Empty".to_string());

            commands.spawn((
                ChildOf(slot_selection),
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(33.3),
                    padding: UiRect::all(Val::Percent(2.0)),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                BackgroundColor(DARK_GREY.into()),
                children![
                    (
                        Text::new(slot.to_string()),
                        button_text_font.clone(),
                        TextColor(TEXT_COLOR),
                    ),
                    (
                        TimePlayed,
                        slot,
                        Text::new(time_played),
                        button_text_font.clone(),
                        TextColor(TEXT_COLOR),
                    ),
                    (
                        Button,
                        StartButton,
                        slot,
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Percent(30.0),
                            height: Val::Percent(80.0),
                            right: Val::Percent(2.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        children![(
                            Text::new("Play"),
                            button_text_font.clone(),
                            TextColor(TEXT_COLOR),
                        )]
                    ),
                ],
            ));
        });

    // Navigation
    commands.spawn((
        ChildOf(root_node),
        Node {
            width: Val::Percent(50.0),
            height: Val::Percent(10.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(SLATE_GRAY.into()),
        children![(
            Button,
            Node {
                width: Val::Px(300.0),
                height: Val::Px(65.0),
                margin: UiRect::all(Val::Px(20.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(NORMAL_BUTTON),
            MenuButtonAction::BackToMainMenu,
            children![
                // (ImageNode::new(right_icon), button_icon_node.clone()),
                (
                    Text::new("Back"),
                    button_text_font.clone(),
                    TextColor(TEXT_COLOR),
                ),
            ]
        )],
    ));
}

fn start_game(
    q_start: Query<(&Interaction, &SaveSlot), With<StartButton>>,
    progress_storage: ResMut<Persistent<ProgressStorage>>,
    mut commands: Commands,
) {
    if let Some(save_slot) = q_start.iter().find_map(|(interaction, save_slot)| {
        matches!(interaction, Interaction::Pressed).then_some(save_slot)
    }) {
        let progress = match &progress_storage[*save_slot] {
            Some(progress) => progress.clone(),
            None => Progress::default(),
        };
        commands.insert_resource(progress);
        commands.insert_resource(*save_slot);
        commands.set_state(AppState::Game {
            paused:   false,
            can_move: true,
        });
    }
}

// This system handles changing all buttons color based on mouse interaction
fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, Option<&SelectedOption>),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut background_color, selected) in &mut interaction_query {
        *background_color = match (*interaction, selected) {
            (Interaction::Pressed, _) | (Interaction::None, Some(_)) => PRESSED_BUTTON.into(),
            (Interaction::Hovered, Some(_)) => HOVERED_PRESSED_BUTTON.into(),
            (Interaction::Hovered, None) => HOVERED_BUTTON.into(),
            (Interaction::None, None) => NORMAL_BUTTON.into(),
        }
    }
}

// This system updates the settings when a new value for a setting is selected, and marks
// the button as the one currently selected
fn radio_settings_system(
    q_interaction: Query<
        (Entity, &Interaction, &RadioSetting),
        (Changed<Interaction>, With<Button>),
    >,
    mut q_selected: Query<(Entity, &mut BackgroundColor, &RadioSetting), With<SelectedOption>>,
    mut commands: Commands,
) {
    q_interaction
        .iter()
        .filter_map(|(button, interaction, setting)| {
            matches!(interaction, Interaction::Pressed).then_some((button, setting))
        })
        .for_each(|(current_button, current_setting)| {
            if let Some((previous_button, mut previous_button_color)) = q_selected
                .iter_mut()
                .filter(|(button, ..)| *button != current_button)
                .find_map(|(button, color, previous_setting)| {
                    (previous_setting == current_setting).then_some((button, color))
                })
            {
                *previous_button_color = NORMAL_BUTTON.into();
                commands.entity(previous_button).remove::<SelectedOption>();
                commands.entity(current_button).insert(SelectedOption);
            }
        });
}

fn save_settings(
    mut settings: ResMut<Persistent<Settings>>,
    q_radio_settings: Query<(&RadioSetting, &RadioValue), With<SelectedOption>>,
) {
    for (setting, RadioValue(value)) in q_radio_settings {
        match setting {
            RadioSetting::Sound => settings.sound_vol = 0.1 * *value as f32,
            RadioSetting::Music => settings.music_vol = 0.1 * *value as f32,
        }
    }
    settings.persist().expect("Settings should be loaded");
}
