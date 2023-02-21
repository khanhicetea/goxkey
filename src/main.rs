mod config;
mod hotkey;
mod input;
mod platform;
mod ui;

use druid::{AppLauncher, ExtEventSink, Target, WindowDesc};
use input::{get_key_from_char, INPUT_STATE, KEYBOARD_LAYOUT_CHARACTER_MAP, PREDEFINED_CHARS};
use log::debug;
use once_cell::sync::OnceCell;
use platform::{
    run_event_listener, send_backspace, send_string, Handle, KeyModifier, KEY_DELETE, KEY_ENTER,
    KEY_ESCAPE, KEY_SPACE, KEY_TAB,
};
use rdev::{EventType, Keyboard, KeyboardState};
use std::{collections::HashMap, thread};
use ui::{UIDataAdapter, UPDATE_UI};

static UI_EVENT_SINK: OnceCell<ExtEventSink> = OnceCell::new();

fn do_transform_keys(handle: Handle, is_delete: bool) -> bool {
    unsafe {
        let output = INPUT_STATE.transform_keys();
        debug!("Transformed: {:?}", output);
        if INPUT_STATE.should_send_keyboard_event(&output) || is_delete {
            let backspace_count = INPUT_STATE.get_backspace_count(is_delete);
            debug!("Backspace count: {}", backspace_count);
            _ = send_backspace(handle, backspace_count);
            _ = send_string(handle, &output);
            INPUT_STATE.replace(output);
            return true;
        }
    }

    return false;
}

fn event_handler(handle: Handle, keycode: Option<char>, modifiers: KeyModifier) -> bool {
    unsafe {
        match keycode {
            Some(keycode) => {
                if INPUT_STATE.get_hotkey().is_match(modifiers, &keycode) {
                    INPUT_STATE.toggle_vietnamese();
                    if let Some(event_sink) = UI_EVENT_SINK.get() {
                        _ = event_sink.submit_command(UPDATE_UI, (), Target::Auto);
                    }
                    return true;
                }

                if INPUT_STATE.is_enabled() {
                    match keycode {
                        KEY_ENTER | KEY_TAB | KEY_SPACE | KEY_ESCAPE => {
                            INPUT_STATE.new_word();
                        }
                        KEY_DELETE => {
                            INPUT_STATE.pop();
                            if !INPUT_STATE.is_buffer_empty() {
                                return do_transform_keys(handle, true);
                            } else {
                                INPUT_STATE.clear();
                            }
                        }
                        c => {
                            if "()[]{}<>/\\!@#$%^&*-_=+|~`'\"".contains(c)
                                || (c.is_numeric() && modifiers.is_shift())
                            {
                                // If special characters detected, dismiss the current tracking word
                                INPUT_STATE.new_word();
                            } else {
                                // Otherwise, process the character
                                if modifiers.is_super()
                                    || modifiers.is_control()
                                    || modifiers.is_alt()
                                {
                                    INPUT_STATE.new_word();
                                } else if INPUT_STATE.is_tracking() {
                                    INPUT_STATE.push(if modifiers.is_shift() {
                                        c.to_ascii_uppercase()
                                    } else {
                                        c
                                    });
                                    if INPUT_STATE.should_transform_keys(&c) {
                                        return do_transform_keys(handle, false);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            None => {
                INPUT_STATE.new_word();
            }
        }
    }
    false
}

fn main() {
    env_logger::init();

    let mut map = HashMap::new();
    let mut kb = Keyboard::new().unwrap();
    for c in PREDEFINED_CHARS {
        let key = rdev::EventType::KeyPress(get_key_from_char(c));
        if let Some(s) = kb.add(&key) {
            let ch = s.chars().last().unwrap();
            map.insert(c, ch);
        }
    }
    _ = KEYBOARD_LAYOUT_CHARACTER_MAP.set(map);

    let win = WindowDesc::new(ui::main_ui_builder)
        .title("gõkey")
        .window_size((320.0, 234.0))
        .resizable(false);
    let app = AppLauncher::with_window(win);
    let event_sink = app.get_external_handle();
    _ = UI_EVENT_SINK.set(event_sink);

    thread::spawn(|| {
        run_event_listener(&event_handler);
    });

    _ = app.launch(UIDataAdapter::new());
}
