//! Window-scoped game input, independent from editor shortcuts and text fields.
use super::play_mode::{PlayModeAction, PlayState, PreviewWindow};
use bevy::{
    input::{
        InputSystems,
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    },
    prelude::*,
    window::{CursorGrabMode, CursorOptions, WindowCloseRequested},
};

#[derive(Resource, Default)]
pub struct GameInput {
    pub keys: ButtonInput<KeyCode>,
    pub mouse_buttons: ButtonInput<MouseButton>,
    pub mouse_delta: Vec2,
    pub scroll_delta: Vec2,
    pub active: bool,
    captured: bool,
    window: Option<Entity>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<GameInput>()
        .add_systems(PreUpdate, collect_game_input.after(InputSystems));
}
fn collect_game_input(
    state: Res<PlayState>,
    windows: Query<(Entity, &Window), With<PreviewWindow>>,
    mut keyboard: MessageReader<KeyboardInput>,
    mut buttons: MessageReader<MouseButtonInput>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    mut close: MessageReader<WindowCloseRequested>,
    mut actions: MessageWriter<PlayModeAction>,
    mut input: ResMut<GameInput>,
    mut cursors: Query<&mut CursorOptions, With<PreviewWindow>>,
) {
    input.keys.clear();
    input.mouse_buttons.clear();
    input.mouse_delta = Vec2::ZERO;
    input.scroll_delta = Vec2::ZERO;
    let preview = windows.single().ok();
    let id = preview.map(|(id, _)| id);
    if input.window != id {
        *input = GameInput {
            window: id,
            captured: true,
            ..default()
        };
    }
    for event in close.read() {
        if Some(event.window) == id {
            actions.write(PlayModeAction::Stop);
        }
    }
    if !preview.is_some_and(|(_, window)| window.focused) || *state != PlayState::Playing {
        input.active = false;
        input.keys.reset_all();
        input.mouse_buttons.reset_all();
        keyboard.clear();
        buttons.clear();
        motion.clear();
        wheel.clear();
        return;
    }
    for event in buttons.read().filter(|event| Some(event.window) == id) {
        if event.state.is_pressed() {
            input.captured = true;
            input.mouse_buttons.press(event.button);
        } else {
            input.mouse_buttons.release(event.button);
        }
    }
    for event in keyboard.read().filter(|event| Some(event.window) == id) {
        if event.state.is_pressed() && event.key_code == KeyCode::F8 {
            actions.write(PlayModeAction::Stop);
        }
        if event.state.is_pressed() && event.key_code == KeyCode::Escape {
            input.captured = false;
            if let Some(id) = id {
                if let Ok(mut cursor) = cursors.get_mut(id) {
                    cursor.grab_mode = CursorGrabMode::None;
                    cursor.visible = true;
                }
            }
        }
        if !input.captured {
            continue;
        }
        if event.state.is_pressed() {
            if !event.repeat {
                input.keys.press(event.key_code);
            }
        } else {
            input.keys.release(event.key_code);
        }
    }
    input.active = input.captured;
    if input.active {
        for event in motion.read() {
            input.mouse_delta += event.delta;
        }
        for event in wheel.read().filter(|event| Some(event.window) == id) {
            input.scroll_delta += Vec2::new(event.x, event.y);
        }
    } else {
        input.keys.reset_all();
        input.mouse_buttons.reset_all();
        motion.clear();
        wheel.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{
        ecs::system::RunSystemOnce,
        input::{ButtonState, keyboard::Key},
    };
    #[test]
    fn input_ignores_editor_window_and_clears_when_game_loses_focus() {
        let mut world = World::new();
        world.insert_resource(PlayState::Playing);
        world.init_resource::<GameInput>();
        world.init_resource::<Messages<KeyboardInput>>();
        world.init_resource::<Messages<MouseButtonInput>>();
        world.init_resource::<Messages<MouseMotion>>();
        world.init_resource::<Messages<MouseWheel>>();
        world.init_resource::<Messages<WindowCloseRequested>>();
        world.init_resource::<Messages<PlayModeAction>>();
        let editor = world.spawn(Window::default()).id();
        let game = world
            .spawn((
                PreviewWindow,
                Window {
                    focused: true,
                    ..default()
                },
            ))
            .id();
        let event = |window, key_code| KeyboardInput {
            window,
            key_code,
            logical_key: Key::Character("w".into()),
            state: ButtonState::Pressed,
            text: Some("w".into()),
            repeat: false,
        };
        world.write_message(event(editor, KeyCode::KeyW));
        world.run_system_once(collect_game_input).unwrap();
        assert!(!world.resource::<GameInput>().keys.pressed(KeyCode::KeyW));
        world.resource_mut::<Messages<KeyboardInput>>().clear();
        world.write_message(event(game, KeyCode::KeyW));
        world.run_system_once(collect_game_input).unwrap();
        assert!(world.resource::<GameInput>().keys.pressed(KeyCode::KeyW));
        world.get_mut::<Window>(game).unwrap().focused = false;
        world.run_system_once(collect_game_input).unwrap();
        assert!(!world.resource::<GameInput>().active);
        assert!(!world.resource::<GameInput>().keys.pressed(KeyCode::KeyW));
    }
}
