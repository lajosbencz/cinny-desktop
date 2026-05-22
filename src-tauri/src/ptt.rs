use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

pub struct PttState {
    binding: Arc<Mutex<Option<String>>>,
    active: Arc<AtomicBool>,
    capture_mode: Arc<AtomicBool>,
    hook: Arc<Mutex<Option<monio::Hook>>>,
}

impl PttState {
    pub fn new() -> Self {
        Self {
            binding: Arc::new(Mutex::new(None)),
            active: Arc::new(AtomicBool::new(false)),
            capture_mode: Arc::new(AtomicBool::new(false)),
            hook: Arc::new(Mutex::new(None)),
        }
    }
}

fn is_modifier_key(key: monio::Key) -> bool {
    matches!(
        key,
        monio::Key::ShiftLeft
            | monio::Key::ShiftRight
            | monio::Key::ControlLeft
            | monio::Key::ControlRight
            | monio::Key::AltLeft
            | monio::Key::AltRight
            | monio::Key::MetaLeft
            | monio::Key::MetaRight
    )
}

/// Build the modifier prefix from the event mask (fixed order: Shift+Ctrl+Alt+Meta+).
fn modifier_prefix_from_mask(mask: u32) -> String {
    let mut prefix = String::new();
    if mask & monio::state::MASK_SHIFT != 0 {
        prefix.push_str("Shift+");
    }
    if mask & monio::state::MASK_CTRL != 0 {
        prefix.push_str("Ctrl+");
    }
    if mask & monio::state::MASK_ALT != 0 {
        prefix.push_str("Alt+");
    }
    if mask & monio::state::MASK_META != 0 {
        prefix.push_str("Meta+");
    }
    prefix
}

/// Parse a modifier prefix string back into a mask.
fn mask_from_prefix(prefix: &str) -> u32 {
    let mut mask: u32 = 0;
    for part in prefix.split('+') {
        match part {
            "Shift" => mask |= monio::state::MASK_SHIFT,
            "Ctrl" => mask |= monio::state::MASK_CTRL,
            "Alt" => mask |= monio::state::MASK_ALT,
            "Meta" => mask |= monio::state::MASK_META,
            _ => {}
        }
    }
    mask
}

fn monio_key_to_string(key: monio::Key) -> String {
    format!("{:?}", key)
}

fn string_to_monio_key(name: &str) -> Option<monio::Key> {
    match name {
        "Space" => Some(monio::Key::Space),
        "Tab" => Some(monio::Key::Tab),
        "Enter" => Some(monio::Key::Enter),
        "Escape" => Some(monio::Key::Escape),
        "Backspace" => Some(monio::Key::Backspace),
        "Delete" => Some(monio::Key::Delete),
        "Home" => Some(monio::Key::Home),
        "End" => Some(monio::Key::End),
        "PageUp" => Some(monio::Key::PageUp),
        "PageDown" => Some(monio::Key::PageDown),
        "ArrowUp" => Some(monio::Key::ArrowUp),
        "ArrowDown" => Some(monio::Key::ArrowDown),
        "ArrowLeft" => Some(monio::Key::ArrowLeft),
        "ArrowRight" => Some(monio::Key::ArrowRight),
        "CapsLock" => Some(monio::Key::CapsLock),
        "F1" => Some(monio::Key::F1),
        "F2" => Some(monio::Key::F2),
        "F3" => Some(monio::Key::F3),
        "F4" => Some(monio::Key::F4),
        "F5" => Some(monio::Key::F5),
        "F6" => Some(monio::Key::F6),
        "F7" => Some(monio::Key::F7),
        "F8" => Some(monio::Key::F8),
        "F9" => Some(monio::Key::F9),
        "F10" => Some(monio::Key::F10),
        "F11" => Some(monio::Key::F11),
        "F12" => Some(monio::Key::F12),
        "KeyA" => Some(monio::Key::KeyA),
        "KeyB" => Some(monio::Key::KeyB),
        "KeyC" => Some(monio::Key::KeyC),
        "KeyD" => Some(monio::Key::KeyD),
        "KeyE" => Some(monio::Key::KeyE),
        "KeyF" => Some(monio::Key::KeyF),
        "KeyG" => Some(monio::Key::KeyG),
        "KeyH" => Some(monio::Key::KeyH),
        "KeyI" => Some(monio::Key::KeyI),
        "KeyJ" => Some(monio::Key::KeyJ),
        "KeyK" => Some(monio::Key::KeyK),
        "KeyL" => Some(monio::Key::KeyL),
        "KeyM" => Some(monio::Key::KeyM),
        "KeyN" => Some(monio::Key::KeyN),
        "KeyO" => Some(monio::Key::KeyO),
        "KeyP" => Some(monio::Key::KeyP),
        "KeyQ" => Some(monio::Key::KeyQ),
        "KeyR" => Some(monio::Key::KeyR),
        "KeyS" => Some(monio::Key::KeyS),
        "KeyT" => Some(monio::Key::KeyT),
        "KeyU" => Some(monio::Key::KeyU),
        "KeyV" => Some(monio::Key::KeyV),
        "KeyW" => Some(monio::Key::KeyW),
        "KeyX" => Some(monio::Key::KeyX),
        "KeyY" => Some(monio::Key::KeyY),
        "KeyZ" => Some(monio::Key::KeyZ),
        "Num0" => Some(monio::Key::Num0),
        "Num1" => Some(monio::Key::Num1),
        "Num2" => Some(monio::Key::Num2),
        "Num3" => Some(monio::Key::Num3),
        "Num4" => Some(monio::Key::Num4),
        "Num5" => Some(monio::Key::Num5),
        "Num6" => Some(monio::Key::Num6),
        "Num7" => Some(monio::Key::Num7),
        "Num8" => Some(monio::Key::Num8),
        "Num9" => Some(monio::Key::Num9),
        "Minus" => Some(monio::Key::Minus),
        "Equal" => Some(monio::Key::Equal),
        "BracketLeft" => Some(monio::Key::BracketLeft),
        "BracketRight" => Some(monio::Key::BracketRight),
        "Backslash" => Some(monio::Key::Backslash),
        "Semicolon" => Some(monio::Key::Semicolon),
        "Quote" => Some(monio::Key::Quote),
        "Comma" => Some(monio::Key::Comma),
        "Period" => Some(monio::Key::Period),
        "Slash" => Some(monio::Key::Slash),
        "Grave" => Some(monio::Key::Grave),
        "ShiftLeft" => Some(monio::Key::ShiftLeft),
        "ShiftRight" => Some(monio::Key::ShiftRight),
        "ControlLeft" => Some(monio::Key::ControlLeft),
        "ControlRight" => Some(monio::Key::ControlRight),
        "AltLeft" => Some(monio::Key::AltLeft),
        "AltRight" => Some(monio::Key::AltRight),
        "MetaLeft" => Some(monio::Key::MetaLeft),
        "MetaRight" => Some(monio::Key::MetaRight),
        _ => None,
    }
}

fn mouse_button_to_string(btn: monio::Button) -> String {
    match btn {
        monio::Button::Left => "Left".to_string(),
        monio::Button::Right => "Right".to_string(),
        monio::Button::Middle => "Middle".to_string(),
        monio::Button::Button4 => "Button4".to_string(),
        monio::Button::Button5 => "Button5".to_string(),
        monio::Button::Unknown(n) => format!("x{}", n),
    }
}

fn string_to_monio_button(name: &str) -> Option<monio::Button> {
    match name {
        "Left" => Some(monio::Button::Left),
        "Right" => Some(monio::Button::Right),
        "Middle" => Some(monio::Button::Middle),
        "Button4" => Some(monio::Button::Button4),
        "Button5" => Some(monio::Button::Button5),
        other => {
            let n: u8 = other.strip_prefix('x')?.parse().ok()?;
            Some(monio::Button::Unknown(n))
        }
    }
}

enum PttTrigger {
    Key { key: monio::Key, modifiers: u32 },
    Mouse(monio::Button),
}

fn parse_binding(binding: &str) -> Option<PttTrigger> {
    if let Some(combo) = binding.strip_prefix("key:") {
        let last_plus = combo.rfind('+');
        let (prefix, key_name) = match last_plus {
            Some(pos) => (&combo[..=pos], &combo[pos + 1..]),
            None => ("", combo),
        };
        let key = string_to_monio_key(key_name)?;
        let modifiers = mask_from_prefix(prefix);
        Some(PttTrigger::Key { key, modifiers })
    } else if let Some(name) = binding.strip_prefix("mouse:") {
        string_to_monio_button(name).map(PttTrigger::Mouse)
    } else {
        None
    }
}

fn start_hook(app: &AppHandle, state: &PttState) {
    let mut hook_guard = state.hook.lock().unwrap();
    if hook_guard.is_some() {
        return;
    }

    let capture_mode = state.capture_mode.clone();
    let active = state.active.clone();
    let binding = state.binding.clone();
    let app_handle = app.clone();

    let hook = monio::Hook::new();
    let _ = hook.run_async(move |event: &monio::Event| {
        // Capture mode: send the first non-modifier press event to the frontend
        if capture_mode.load(Ordering::SeqCst) {
            let captured = match event.event_type {
                monio::EventType::KeyPressed => event.keyboard.as_ref().and_then(|kb| {
                    if is_modifier_key(kb.key) {
                        return None;
                    }
                    let prefix = modifier_prefix_from_mask(event.mask);
                    Some(format!("key:{}{}", prefix, monio_key_to_string(kb.key)))
                }),
                monio::EventType::MousePressed => event
                    .mouse
                    .as_ref()
                    .and_then(|m| m.button)
                    .map(|btn| format!("mouse:{}", mouse_button_to_string(btn))),
                _ => None,
            };

            if let Some(b) = captured {
                capture_mode.store(false, Ordering::SeqCst);
                let _ = app_handle.emit("ptt-captured", b);
            }
            return;
        }

        // PTT mode: match against the configured binding
        if !active.load(Ordering::SeqCst) {
            return;
        }

        let current_binding = binding.lock().unwrap().clone();
        let Some(ref binding_str) = current_binding else {
            return;
        };
        let Some(trigger) = parse_binding(binding_str) else {
            return;
        };

        let modifier_mask = event.mask
            & (monio::state::MASK_SHIFT
                | monio::state::MASK_CTRL
                | monio::state::MASK_ALT
                | monio::state::MASK_META);

        let pressed = match (&trigger, &event.event_type) {
            (PttTrigger::Key { key, modifiers }, monio::EventType::KeyPressed) => event
                .keyboard
                .as_ref()
                .filter(|kb| kb.key == *key && modifier_mask == *modifiers)
                .map(|_| true),
            (PttTrigger::Key { key, .. }, monio::EventType::KeyReleased) => event
                .keyboard
                .as_ref()
                .filter(|kb| kb.key == *key)
                .map(|_| false),
            (PttTrigger::Mouse(target), monio::EventType::MousePressed) => event
                .mouse
                .as_ref()
                .and_then(|m| m.button)
                .filter(|btn| btn == target)
                .map(|_| true),
            (PttTrigger::Mouse(target), monio::EventType::MouseReleased) => event
                .mouse
                .as_ref()
                .and_then(|m| m.button)
                .filter(|btn| btn == target)
                .map(|_| false),
            _ => None,
        };

        if let Some(is_pressed) = pressed {
            let _ = app_handle.emit("ptt-state", is_pressed);
        }
    });

    *hook_guard = Some(hook);
}

#[tauri::command]
pub fn ptt_register(
    app: AppHandle,
    state: State<'_, PttState>,
    binding: String,
) -> Result<(), String> {
    parse_binding(&binding).ok_or_else(|| format!("Invalid binding: {}", binding))?;

    *state.binding.lock().unwrap() = Some(binding);
    state.active.store(true, Ordering::SeqCst);
    start_hook(&app, &state);

    Ok(())
}

#[tauri::command]
pub fn ptt_unregister(state: State<'_, PttState>) -> Result<(), String> {
    state.active.store(false, Ordering::SeqCst);
    *state.binding.lock().unwrap() = None;
    Ok(())
}

#[tauri::command]
pub fn ptt_start_capture(app: AppHandle, state: State<'_, PttState>) -> Result<(), String> {
    state.capture_mode.store(true, Ordering::SeqCst);
    start_hook(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn ptt_stop_capture(state: State<'_, PttState>) -> Result<(), String> {
    state.capture_mode.store(false, Ordering::SeqCst);
    Ok(())
}
