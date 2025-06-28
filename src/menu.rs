#![allow(clippy::type_complexity)]
use bevy::{
    color::palettes::css::*,
    ecs::{spawn::SpawnWith, system::SystemId},
    prelude::*,
};
use bevy_persistent::Persistent;

use super::{AppState, Settings, despawn_screen};

const TEXT_COLOR: Color = Color::Srgba(WHITE_SMOKE);

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const HOVERED_PRESSED_BUTTON: Color = Color::srgb(0.25, 0.65, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum MenuState {
    Main,
    Settings,
    #[default]
    Disabled,
}

#[derive(Component)]
struct OnMainMenuScreen;

#[derive(Component)]
struct OnSettingsScreen;

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

#[derive(Component, PartialEq, Clone, Copy)]
enum RadioSetting {
    Sound,
    Music,
}

#[derive(Component, PartialEq, Clone, Copy)]
struct RadioValue(u32);

#[derive(Resource)]
struct SaveSystemId(SystemId);

pub fn menu_plugin(app: &mut App) {
    let save_system_id = app.register_system(save_settings);

    app.init_state::<MenuState>()
        .add_systems(OnEnter(AppState::Menu), menu_setup)
        .add_systems(OnEnter(MenuState::Main), main_menu_setup)
        .add_systems(OnExit(MenuState::Main), despawn_screen::<OnMainMenuScreen>)
        .add_systems(OnEnter(MenuState::Settings), settings_menu_setup)
        .add_systems(
            OnExit(MenuState::Settings),
            despawn_screen::<OnSettingsScreen>,
        )
        .add_systems(
            Update,
            (menu_action, button_system).run_if(in_state(AppState::Menu)),
        )
        .add_systems(
            Update,
            radio_settings_system.run_if(in_state(MenuState::Settings)),
        )
        .insert_resource(SaveSystemId(save_system_id));
}

fn menu_action(
    interaction_query: Query<
        (&Interaction, &MenuButtonAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut app_exit_events: EventWriter<AppExit>,
    mut menu_state: ResMut<NextState<MenuState>>,
    mut game_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
    save_system_id: Res<SaveSystemId>,
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
                game_state.set(AppState::Game);
                menu_state.set(MenuState::Disabled);
            }
            MenuButtonAction::Settings => menu_state.set(MenuState::Settings),
            MenuButtonAction::SaveSettings => {
                // settings.persist().expect("Failed writing settings to disk");
                commands.run_system(save_system_id.0);
                menu_state.set(MenuState::Main);
            }
            MenuButtonAction::BackToMainMenu => menu_state.set(MenuState::Main),
        }
    }
}

fn menu_setup(mut menu_state: ResMut<NextState<MenuState>>) {
    menu_state.set(MenuState::Main);
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
            OnMainMenuScreen,
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
            OnSettingsScreen,
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
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
                        .into_iter()
                        .for_each(|volume_setting| {
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
                                RadioValue(volume_setting),
                            ));
                            if music_vol == volume_setting {
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
                    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
                        .into_iter()
                        .for_each(|volume_setting| {
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
                                RadioValue(volume_setting),
                            ));
                            if sound_vol == volume_setting {
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
    interaction_query: Query<
        (&Interaction, Entity, &RadioSetting),
        (Changed<Interaction>, With<Button>),
    >,
    mut selected_query: Query<(Entity, &mut BackgroundColor, &RadioSetting), With<SelectedOption>>,
    mut commands: Commands,
) {
    for (interaction, current_button, current_radio_type) in &interaction_query {
        let (previous_button, mut previous_button_color, _setvol) = selected_query
            .iter_mut()
            .find(|(_entity, _color, previous_radio_type)| {
                previous_radio_type == &current_radio_type
            })
            .unwrap();

        if *interaction != Interaction::Pressed || current_button == previous_button {
            continue;
        }

        *previous_button_color = NORMAL_BUTTON.into();
        commands.entity(previous_button).remove::<SelectedOption>();
        commands.entity(current_button).insert(SelectedOption);
    }
}

fn save_settings(
    mut settings: ResMut<Persistent<Settings>>,
    settings_query: Query<(&RadioSetting, &RadioValue), With<SelectedOption>>,
) {
    for (setting, value) in settings_query {
        match setting {
            RadioSetting::Sound => settings.sound_vol = value.0,
            RadioSetting::Music => settings.music_vol = value.0,
        }
    }
    settings.persist().expect("Failed writing settings to disk");
}