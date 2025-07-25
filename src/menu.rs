#![allow(clippy::type_complexity)]
use bevy::{color::palettes::css::*, ecs::spawn::SpawnWith, prelude::*};
use bevy_persistent::Persistent;

use crate::{AppState, Settings, StandardFont, despawn_screen, game::effects::*, progress::*};

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
    Fading,
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
enum NavigationAction {
    Play,
    Settings,
    Quit,
    MainMenu,
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
        .add_systems(OnEnter(MenuState::Main), setup_main_menu)
        .add_systems(OnExit(MenuState::Main), despawn_screen::<OnMainMenu>)
        .add_systems(OnEnter(MenuState::Settings), setup_settings)
        .add_systems(OnExit(MenuState::Settings), despawn_screen::<OnSettings>)
        .add_systems(OnEnter(MenuState::Data), setup_data_menu)
        .add_systems(OnExit(MenuState::Data), despawn_screen::<OnData>)
        .add_systems(
            Update,
            (navigation_action, update_button_color).run_if(in_state(AppState::Menu)),
        )
        .add_systems(
            Update,
            (update_radio_buttons, save_settings).run_if(in_state(MenuState::Settings)),
        )
        .add_systems(Update, setup_progress.run_if(in_state(MenuState::Data)))
        .add_systems(OnEnter(MenuState::Fading), fade_to_black)
        .add_systems(Update, start_game.run_if(on_event::<FadeIn>));
}

fn navigation_action(
    interaction_query: Query<
        (&Interaction, &NavigationAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut app_exit_events: EventWriter<AppExit>,
    mut menu_state: ResMut<NextState<MenuState>>,
) {
    interaction_query
        .into_iter()
        .filter_map(|(interaction, navigation_action)| {
            matches!(interaction, Interaction::Pressed).then_some(navigation_action)
        })
        .for_each(|navigation_action| match navigation_action {
            NavigationAction::Play => menu_state.set(MenuState::Data),
            NavigationAction::Settings => menu_state.set(MenuState::Settings),
            NavigationAction::Quit => {
                app_exit_events.write(AppExit::Success);
            }
            NavigationAction::MainMenu => menu_state.set(MenuState::Main),
        });
}

fn setup_main_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    font: Res<StandardFont>,
) {
    let title = asset_server.load("title.png");

    let main_menu_root = commands
        .spawn((
            Name::new("Main Menu"),
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

    commands.spawn((
        Name::new("Title"),
        ChildOf(main_menu_root),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(48.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(BLACK.into()),
        children![(
            Node {
                aspect_ratio: Some(2.0),
                max_height: Val::Percent(100.0),
                ..default()
            },
            ImageNode::new(title)
        )],
    ));

    let button_node = Node {
        width: Val::Percent(32.0),
        height: Val::Percent(32.0),
        margin: UiRect::all(Val::Px(8.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };

    let button_text_font = TextFont {
        font: font.clone_weak(),
        font_size: 32.0,
        font_smoothing: bevy::text::FontSmoothing::None,
        ..default()
    };

    commands.spawn((
        Name::new("Main Menu Buttons"),
        ChildOf(main_menu_root),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(48.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            // Display three buttons for each action available from the main menu:
            // - new game
            // - settings
            // - quit
            (
                Button,
                button_node.clone(),
                BackgroundColor(NORMAL_BUTTON),
                NavigationAction::Play,
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
                NavigationAction::Settings,
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
                NavigationAction::Quit,
                children![
                    // (ImageNode::new(exit_icon), button_icon_node),
                    (Text::new("Quit"), button_text_font, TextColor(TEXT_COLOR),),
                ]
            ),
        ],
    ));
}

#[derive(Debug, Component)]
struct SaveButton;

fn setup_settings(
    mut commands: Commands,
    _asset_server: Res<AssetServer>,
    font: Res<StandardFont>,
    settings: Res<Persistent<Settings>>,
) {
    let settings_root = commands
        .spawn((
            Name::new("Settings Menu"),
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
        font: font.clone_weak(),
        font_size: 32.0,
        ..default()
    };

    commands.spawn((
        Name::new("Key Bindings"),
        ChildOf(settings_root),
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
                font: font.clone_weak(),
                font_size: 48.0,
                font_smoothing: bevy::text::FontSmoothing::None,
                ..default()
            },
            TextColor(TEXT_COLOR),
        )],
    ));

    commands.spawn((
        Name::new("Music Volume"),
        ChildOf(settings_root),
        settings_node.clone(),
        BackgroundColor(DARK_GREY.into()),
        Children::spawn((
            Spawn((
                Label,
                Text::new("Music"),
                TextFont {
                    font: font.clone_weak(),
                    font_size: 32.0,
                    font_smoothing: bevy::text::FontSmoothing::None,
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
    ));

    commands.spawn((
        Name::new("Sound Volume"),
        ChildOf(settings_root),
        settings_node.clone(),
        BackgroundColor(DARK_GREY.into()),
        Children::spawn((
            Spawn((
                Label,
                Text::new("Sound"),
                TextFont {
                    font: font.clone_weak(),
                    font_size: 32.0,
                    font_smoothing: bevy::text::FontSmoothing::None,
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
    ));

    commands.spawn((
        Name::new("Navigation"),
        ChildOf(settings_root),
        settings_node.clone(),
        BackgroundColor(SLATE_GRAY.into()),
        children![
            (
                Button,
                button_node.clone(),
                BackgroundColor(NORMAL_BUTTON),
                NavigationAction::MainMenu,
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
                SaveButton,
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
    ));
}

#[derive(Debug, Component)]
struct TimePlayed;

#[derive(Debug, Component)]
struct StartButton;

fn setup_data_menu(
    mut commands: Commands,
    progress_storage: Res<Persistent<ProgressStorage>>,
    font: Res<StandardFont>,
) {
    let button_text_font = TextFont {
        font: font.clone_weak(),
        font_size: 32.0,
        font_smoothing: bevy::text::FontSmoothing::None,
        ..default()
    };

    let data_root = commands
        .spawn((
            Name::new("Data Menu"),
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
            Name::new("Slot Selection"),
            ChildOf(data_root),
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
                    format!("{:02}:{:02}:{:02}", hours, minutes % 60, seconds % 60)
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
        ChildOf(data_root),
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
            NavigationAction::MainMenu,
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

fn setup_progress(
    q_start: Query<(&Interaction, &SaveSlot), With<StartButton>>,
    progress_storage: Res<Persistent<ProgressStorage>>,
    mut commands: Commands,
) {
    use bevy::platform::time::Instant;
    if let Some(save_slot) = q_start.iter().find_map(|(interaction, save_slot)| {
        matches!(interaction, Interaction::Pressed).then_some(save_slot)
    }) {
        let progress = match progress_storage.get_slot(*save_slot) {
            Some(progress) => progress.clone(),
            None => Progress::default(),
        };
        commands.insert_resource(progress);
        commands.insert_resource(*save_slot);
        commands.insert_resource(TimePlayedStart(Instant::now()));
        commands.set_state(MenuState::Fading);
    }
}

fn start_game(mut app_state: ResMut<NextState<AppState>>) {
    app_state.set(AppState::Game {
        paused:   false,
        can_move: false,
    });
}

fn update_button_color(
    mut interaction_query: Query<
        (&mut BackgroundColor, &Interaction, Has<SelectedOption>),
        (Changed<Interaction>, With<Button>),
    >,
) {
    interaction_query
        .iter_mut()
        .for_each(|(mut background_color, interaction, selected)| {
            *background_color = match (*interaction, selected) {
                (Interaction::Pressed, _) | (Interaction::None, true) => PRESSED_BUTTON.into(),
                (Interaction::Hovered, true) => HOVERED_PRESSED_BUTTON.into(),
                (Interaction::Hovered, false) => HOVERED_BUTTON.into(),
                (Interaction::None, false) => NORMAL_BUTTON.into(),
            }
        });
}

// This system updates the settings when a new value for a setting is selected, and marks
// the button as the one currently selected
fn update_radio_buttons(
    q_interaction: Query<
        (Entity, &Interaction, &RadioSetting),
        (Changed<Interaction>, With<Button>),
    >,
    mut q_selected: Query<(Entity, &mut BackgroundColor, &RadioSetting), With<SelectedOption>>,
    mut commands: Commands,
) {
    q_interaction
        .into_iter()
        .filter_map(|(button, interaction, setting)| {
            matches!(interaction, Interaction::Pressed).then_some((button, setting))
        })
        .for_each(|(current_button, current_setting)| {
            if let Some((previous_button, mut previous_button_color)) = q_selected
                .iter_mut()
                .filter(|(button, ..)| *button != current_button)
                .find_map(|(button, color, setting)| {
                    (setting == current_setting).then_some((button, color))
                })
            {
                *previous_button_color = NORMAL_BUTTON.into();
                commands.entity(previous_button).remove::<SelectedOption>();
                commands.entity(current_button).insert(SelectedOption);
            }
        });
}

fn save_settings(
    save_button: Single<&Interaction, (Changed<Interaction>, With<SaveButton>)>,
    q_radio_settings: Query<(&RadioSetting, &RadioValue), With<SelectedOption>>,
    mut settings: ResMut<Persistent<Settings>>,
    mut menu_state: ResMut<NextState<MenuState>>,
) {
    if !matches!(save_button.into_inner(), Interaction::Pressed) {
        return;
    }

    q_radio_settings
        .into_iter()
        .for_each(|(setting, RadioValue(value))| match setting {
            RadioSetting::Sound => settings.sound_vol = 0.1 * *value as f32,
            RadioSetting::Music => settings.music_vol = 0.1 * *value as f32,
        });

    settings.persist().expect("Settings should be loaded");

    menu_state.set(MenuState::Main);
}
