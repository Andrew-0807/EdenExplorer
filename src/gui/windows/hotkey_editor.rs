//! Keyboard shortcuts tab: lists every rebindable action and lets the user
//! capture a new key combination for it.
//!
//! Capture works in two steps. `intercept_capture` runs early in the frame,
//! takes the first key press out of the input events (so neither the action it
//! maps to nor Esc-closes-settings sees it) and parks it in the editor state.
//! `draw` then applies it to the overrides.

use crate::core::i18n::I18n;
use crate::core::utils::widgets::eden_button;
use crate::gui::keymap::{self, Action, KeyChord, KeyMap, Overrides};
use crate::gui::theme::ThemePalette;
use crate::gui::windows::settings::{
    body_size, setting_row_keywords, settings_button, settings_section,
};
use eframe::egui;
use egui::RichText;

/// Width of the chips and buttons column.
const HOTKEY_CONTROL_WIDTH: f32 = 400.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    /// The captured chord becomes the only chord of the action.
    Replace,
    /// The captured chord is added next to the existing ones.
    Add,
}

#[derive(Clone, Copy, Debug)]
struct Conflict {
    chord: KeyChord,
    other: Action,
}

#[derive(Clone, Default)]
struct EditorState {
    capturing: Option<(Action, Mode)>,
    /// Key press taken from the input this frame, applied by `draw`.
    captured: Option<(KeyChord, egui::Key)>,
    conflict: Option<Conflict>,
}

fn state_id() -> egui::Id {
    egui::Id::new("hotkey_editor_state")
}

fn load(ctx: &egui::Context) -> EditorState {
    ctx.data(|d| d.get_temp::<EditorState>(state_id()))
        .unwrap_or_default()
}

fn store(ctx: &egui::Context, state: EditorState) {
    ctx.data_mut(|d| d.insert_temp(state_id(), state));
}

/// While a row waits for a key combination, moves the first key press out of
/// the input so nothing else reacts to it. Call before any widget reads keys.
pub fn intercept_capture(ctx: &egui::Context) {
    let mut state = load(ctx);
    if state.capturing.is_none() {
        return;
    }
    let taken = ctx.input_mut(|i| take_chord(&mut i.events));
    if taken.is_some() {
        state.captured = taken;
        store(ctx, state);
    }
}

/// Removes key presses from `events` (modifier keys stay) and returns the first
/// one that forms a chord. Modifier-only presses are ignored so the chord
/// completes on the first non-modifier key with the modifiers held then.
fn take_chord(events: &mut Vec<egui::Event>) -> Option<(KeyChord, egui::Key)> {
    let mut found = None;
    events.retain(|event| {
        if let egui::Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } = event
            && !keymap::is_modifier_key(*key)
        {
            if found.is_none() {
                found = KeyChord::from_key_event(*key, *modifiers).map(|c| (c, *key));
            }
            return false;
        }
        true
    });
    found
}

fn save(overrides: &mut Overrides, map: &KeyMap) {
    *overrides = map.to_overrides();
}

/// Binds `chord` to `action`. `swap` also fixes the action that owned the chord.
fn apply(overrides: &mut Overrides, action: Action, mode: Mode, chord: KeyChord, swap: bool) {
    let mut map = KeyMap::from_overrides(overrides);
    let old: Vec<KeyChord> = map.chords(action).to_vec();
    if swap && let Some(other) = map.action_for(chord) {
        let mut other_chords: Vec<KeyChord> = map.chords(other).to_vec();
        let replacement = (mode == Mode::Replace)
            .then(|| old.first().copied())
            .flatten();
        if let Some(pos) = other_chords.iter().position(|c| *c == chord) {
            match replacement {
                Some(r) if !other_chords.contains(&r) => other_chords[pos] = r,
                _ => {
                    other_chords.remove(pos);
                }
            }
        }
        map.set_chords(other, other_chords);
    }
    let new = match mode {
        Mode::Replace => vec![chord],
        Mode::Add => {
            let mut all = old;
            if !all.contains(&chord) {
                all.push(chord);
            }
            all
        }
    };
    map.set_chords(action, new);
    save(overrides, &map);
}

fn reset_action(overrides: &mut Overrides, action: Action) {
    overrides.remove(action.name());
}

fn chords_text(map: &KeyMap, action: Action, i18n: &I18n) -> String {
    let chords = map.chords(action);
    if chords.is_empty() {
        return i18n.tr("hotkey_unbound");
    }
    chords
        .iter()
        .map(KeyChord::to_string)
        .collect::<Vec<_>>()
        .join(" / ")
}

/// Rounded key chip showing one chord (or the capture prompt).
fn key_chip(ui: &mut egui::Ui, palette: &ThemePalette, text: &str, active: bool) {
    let stroke = if active {
        palette.primary
    } else {
        palette.borders_default
    };
    let color = if active {
        palette.text_normal
    } else {
        palette.text_normal.gamma_multiply(0.9)
    };
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        egui::FontId::proportional(body_size(palette) - 1.0),
        color,
    );
    let size = galley.size() + egui::vec2(16.0, 6.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(6),
        palette.text_normal.gamma_multiply(0.07),
        egui::Stroke::new(1.0, stroke),
        egui::StrokeKind::Inside,
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, color);
}

/// One action: its name on the left, chips and buttons on the right.
fn draw_action_row(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    i18n: &I18n,
    overrides: &mut Overrides,
    map: &KeyMap,
    state: &mut EditorState,
    action: Action,
) {
    let capturing_here = state.capturing.map(|(a, _)| a) == Some(action);
    let chords = map.chords(action);
    let keywords = chords_text(map, action, i18n);
    let label = i18n.tr(action.label_key());
    // Controls are added right to left: Reset, +, Change, then the chips.
    setting_row_keywords(
        ui,
        palette,
        &label,
        None,
        &keywords,
        HOTKEY_CONTROL_WIDTH,
        |ui| {
            if capturing_here {
                if eden_button(ui, palette, &i18n.tr("hotkey_cancel")).clicked() {
                    state.capturing = None;
                    state.conflict = None;
                }
                key_chip(ui, palette, &i18n.tr("hotkey_press_keys"), true);
                return;
            }
            if overrides.contains_key(action.name())
                && eden_button(ui, palette, &i18n.tr("hotkey_reset")).clicked()
            {
                reset_action(overrides, action);
            }
            if eden_button(ui, palette, "+").clicked() {
                state.capturing = Some((action, Mode::Add));
                state.conflict = None;
            }
            if eden_button(ui, palette, &i18n.tr("hotkey_change")).clicked() {
                state.capturing = Some((action, Mode::Replace));
                state.conflict = None;
            }
            if chords.is_empty() {
                ui.label(
                    RichText::new(i18n.tr("hotkey_unbound"))
                        .color(palette.text_normal.gamma_multiply(0.6)),
                );
            }
            for chord in chords.iter().rev() {
                key_chip(ui, palette, &chord.to_string(), false);
            }
        },
    );

    if capturing_here && let Some(conflict) = state.conflict {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "{} {} \"{}\"",
                    conflict.chord,
                    i18n.tr("hotkey_conflict_used_by"),
                    i18n.tr(conflict.other.label_key())
                ))
                .color(palette.text_normal),
            );
            if eden_button(ui, palette, &i18n.tr("hotkey_swap")).clicked()
                && let Some((a, mode)) = state.capturing
            {
                apply(overrides, a, mode, conflict.chord, true);
                *state = EditorState::default();
            } else if eden_button(ui, palette, &i18n.tr("hotkey_cancel")).clicked() {
                state.conflict = None;
            }
        });
    }
}

pub fn draw(ui: &mut egui::Ui, palette: &ThemePalette, i18n: &I18n, overrides: &mut Overrides) {
    let ctx = ui.ctx().clone();
    let mut state = load(&ctx);

    if let Some((chord, key)) = state.captured.take()
        && let Some((action, mode)) = state.capturing
    {
        let bare = !chord.ctrl && !chord.shift && !chord.alt;
        if bare && key == egui::Key::Escape {
            state.capturing = None;
        } else if bare && key == egui::Key::Backspace {
            let mut map = KeyMap::from_overrides(overrides);
            map.set_chords(action, Vec::new());
            save(overrides, &map);
            state.capturing = None;
        } else {
            let map = KeyMap::from_overrides(overrides);
            match map.action_for(chord) {
                Some(other) if other != action => {
                    state.conflict = Some(Conflict { chord, other });
                }
                _ => {
                    apply(overrides, action, mode, chord, false);
                    state.capturing = None;
                }
            }
        }
    }

    let map = KeyMap::from_overrides(overrides);
    ui.spacing_mut().item_spacing.y = 0.0;
    ui.vertical(|ui| {
        ui.add_space(4.0);
        if settings_button(ui, palette, &i18n.tr("hotkeys_reset")).is_some_and(|r| r.clicked()) {
            overrides.clear();
            state = EditorState::default();
        }

        for (group_name, actions) in keymap::GROUPS {
            settings_section(ui, palette, Some(&i18n.tr(group_name)), |ui| {
                for &action in *actions {
                    draw_action_row(ui, palette, i18n, overrides, &map, &mut state, action);
                }
            });
        }
    });

    store(&ctx, state);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_event(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn capture_skips_modifier_presses_and_completes_on_real_key() {
        let shift = egui::Modifiers {
            shift: true,
            ..egui::Modifiers::NONE
        };
        let mut events = vec![
            key_event(egui::Key::ShiftLeft, shift),
            key_event(egui::Key::ShiftRight, shift),
        ];
        assert_eq!(take_chord(&mut events), None);
        assert_eq!(events.len(), 2);

        events.push(key_event(egui::Key::Delete, shift));
        let (chord, key) = take_chord(&mut events).unwrap();
        assert_eq!(chord.to_string(), "Shift+Delete");
        assert_eq!(key, egui::Key::Delete);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn apply_replaces_and_reset_restores_default() {
        let mut o = Overrides::new();
        let chord = KeyChord::ctrl(egui::Key::J);
        apply(&mut o, Action::NewTab, Mode::Replace, chord, false);
        assert_eq!(KeyMap::from_overrides(&o).chords(Action::NewTab), &[chord]);
        reset_action(&mut o, Action::NewTab);
        assert!(o.is_empty());
    }

    #[test]
    fn add_keeps_existing_chords() {
        let mut o = Overrides::new();
        let chord = KeyChord::ctrl(egui::Key::J);
        apply(&mut o, Action::Refresh, Mode::Add, chord, false);
        assert_eq!(KeyMap::from_overrides(&o).chords(Action::Refresh).len(), 3);
    }

    #[test]
    fn swap_gives_the_other_action_the_old_chord() {
        let mut o = Overrides::new();
        // NewTab takes CloseTab's Ctrl+W; CloseTab gets NewTab's old Ctrl+T.
        apply(
            &mut o,
            Action::NewTab,
            Mode::Replace,
            KeyChord::ctrl(egui::Key::W),
            true,
        );
        let map = KeyMap::from_overrides(&o);
        assert_eq!(map.chords(Action::NewTab), &[KeyChord::ctrl(egui::Key::W)]);
        assert_eq!(
            map.chords(Action::CloseTab),
            &[KeyChord::ctrl(egui::Key::T)]
        );
    }

    #[test]
    fn swap_in_add_mode_only_unbinds_the_other_action() {
        let mut o = Overrides::new();
        apply(
            &mut o,
            Action::NewTab,
            Mode::Add,
            KeyChord::ctrl(egui::Key::W),
            true,
        );
        let map = KeyMap::from_overrides(&o);
        assert!(map.chords(Action::CloseTab).is_empty());
        assert_eq!(map.chords(Action::NewTab).len(), 2);
    }
}
