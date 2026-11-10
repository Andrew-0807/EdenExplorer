//! Rebindable keyboard shortcuts.
//!
//! `Action` lists every shortcut a user can rebind, `KeyMap` maps each action to
//! one or more `KeyChord`s. Only user overrides are stored in `settings.json`
//! (`Overrides`, action name to chord strings such as "Ctrl+Shift+N"); a missing
//! action uses its default chords. Handlers ask `keymap::pressed(input, Action)`
//! and read the process-wide map, which `sync` refreshes from the settings.

use eframe::egui;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::str::FromStr;
use std::sync::RwLock;

/// A key plus the exact set of modifiers that must be held with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyChord {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: egui::Key,
}

impl KeyChord {
    pub const fn new(ctrl: bool, shift: bool, alt: bool, key: egui::Key) -> Self {
        Self {
            ctrl,
            shift,
            alt,
            key,
        }
    }

    pub const fn key(key: egui::Key) -> Self {
        Self::new(false, false, false, key)
    }

    pub const fn ctrl(key: egui::Key) -> Self {
        Self::new(true, false, false, key)
    }

    pub const fn alt(key: egui::Key) -> Self {
        Self::new(false, false, true, key)
    }

    pub fn modifiers(&self) -> egui::Modifiers {
        egui::Modifiers {
            ctrl: self.ctrl,
            command: self.ctrl,
            shift: self.shift,
            alt: self.alt,
            ..egui::Modifiers::NONE
        }
    }

    fn matches(&self, input: &egui::InputState) -> bool {
        let m = input.modifiers;
        m.ctrl == self.ctrl
            && m.shift == self.shift
            && m.alt == self.alt
            && input.key_pressed(self.key)
    }
}

impl fmt::Display for KeyChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ctrl {
            f.write_str("Ctrl+")?;
        }
        if self.shift {
            f.write_str("Shift+")?;
        }
        if self.alt {
            f.write_str("Alt+")?;
        }
        f.write_str(self.key.name())
    }
}

impl FromStr for KeyChord {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        let mut chord = KeyChord::key(egui::Key::A);
        let mut parts: Vec<&str> = s.split('+').map(str::trim).collect();
        // "Ctrl++" style keys are not representable; the key is the last part.
        let key_name = parts.pop().filter(|p| !p.is_empty()).ok_or(())?;
        for part in parts {
            match part.to_ascii_lowercase().as_str() {
                "ctrl" => chord.ctrl = true,
                "shift" => chord.shift = true,
                "alt" => chord.alt = true,
                _ => return Err(()),
            }
        }
        chord.key = egui::Key::from_name(key_name).ok_or(())?;
        Ok(chord)
    }
}

/// Every shortcut that can be rebound. Context keys (text editing, popup
/// arrows/Enter/Esc, clipboard Ctrl+C/X/V, list arrows) stay hardcoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    ToggleFullscreen,
    ZoxideJump,
    NewTab,
    CloseTab,
    NextTab,
    PreviousTab,
    CopyPath,
    NewFolder,
    Rename,
    Properties,
    OpenSelected,
    Delete,
    PermanentDelete,
    SelectAll,
    Refresh,
    AddressBar,
    NavigateBack,
    NavigateForward,
    NavigateUp,
    SelectFirst,
    SelectLast,
}

/// Settings groups in display order: (locale key, actions).
pub const GROUPS: &[(&str, &[Action])] = &[
    (
        "hotkey_group_application",
        &[Action::ToggleFullscreen, Action::ZoxideJump],
    ),
    (
        "hotkey_group_tabs",
        &[
            Action::NewTab,
            Action::CloseTab,
            Action::NextTab,
            Action::PreviousTab,
        ],
    ),
    (
        "hotkey_group_files_folders",
        &[
            Action::CopyPath,
            Action::NewFolder,
            Action::Rename,
            Action::Properties,
            Action::OpenSelected,
            Action::Delete,
            Action::PermanentDelete,
            Action::SelectAll,
        ],
    ),
    (
        "hotkey_group_navigation",
        &[
            Action::Refresh,
            Action::AddressBar,
            Action::NavigateBack,
            Action::NavigateForward,
            Action::NavigateUp,
            Action::SelectFirst,
            Action::SelectLast,
        ],
    ),
];

impl Action {
    /// Stable name used in `settings.json`.
    pub fn name(self) -> &'static str {
        match self {
            Action::ToggleFullscreen => "ToggleFullscreen",
            Action::ZoxideJump => "ZoxideJump",
            Action::NewTab => "NewTab",
            Action::CloseTab => "CloseTab",
            Action::NextTab => "NextTab",
            Action::PreviousTab => "PreviousTab",
            Action::CopyPath => "CopyPath",
            Action::NewFolder => "NewFolder",
            Action::Rename => "Rename",
            Action::Properties => "Properties",
            Action::OpenSelected => "OpenSelected",
            Action::Delete => "Delete",
            Action::PermanentDelete => "PermanentDelete",
            Action::SelectAll => "SelectAll",
            Action::Refresh => "Refresh",
            Action::AddressBar => "AddressBar",
            Action::NavigateBack => "NavigateBack",
            Action::NavigateForward => "NavigateForward",
            Action::NavigateUp => "NavigateUp",
            Action::SelectFirst => "SelectFirst",
            Action::SelectLast => "SelectLast",
        }
    }

    pub fn all() -> impl Iterator<Item = Action> {
        GROUPS
            .iter()
            .flat_map(|(_, actions)| actions.iter().copied())
    }

    pub fn from_name(name: &str) -> Option<Action> {
        Action::all().find(|a| a.name() == name)
    }

    /// Locale key of the action's description.
    pub fn label_key(self) -> &'static str {
        match self {
            Action::ToggleFullscreen => "hotkey_toggle_fullscreen",
            Action::ZoxideJump => "hotkey_zoxide_jump",
            Action::NewTab => "hotkey_new_tab",
            Action::CloseTab => "hotkey_close_current_tab",
            Action::NextTab => "hotkey_next_tab",
            Action::PreviousTab => "hotkey_previous_tab",
            Action::CopyPath => "hotkey_copy_selected_path",
            Action::NewFolder => "hotkey_new_folder",
            Action::Rename => "hotkey_rename_file",
            Action::Properties => "hotkey_open_properties",
            Action::OpenSelected => "hotkey_run_default_program",
            Action::Delete => "hotkey_delete",
            Action::PermanentDelete => "hotkey_permanent_delete",
            Action::SelectAll => "hotkey_select_all",
            Action::Refresh => "hotkey_refresh_directory",
            Action::AddressBar => "hotkey_select_address_bar",
            Action::NavigateBack => "hotkey_navigate_back",
            Action::NavigateForward => "hotkey_navigate_forward",
            Action::NavigateUp => "hotkey_navigate_up",
            Action::SelectFirst => "hotkey_select_first",
            Action::SelectLast => "hotkey_select_last",
        }
    }

    pub fn defaults(self) -> Vec<KeyChord> {
        use egui::Key;
        match self {
            Action::ToggleFullscreen => vec![KeyChord::key(Key::F1)],
            Action::ZoxideJump => vec![KeyChord::ctrl(Key::K)],
            Action::NewTab => vec![KeyChord::ctrl(Key::T)],
            Action::CloseTab => vec![KeyChord::ctrl(Key::W)],
            Action::NextTab => vec![KeyChord::ctrl(Key::Tab)],
            Action::PreviousTab => vec![KeyChord::new(true, true, false, Key::Tab)],
            Action::CopyPath => vec![KeyChord::new(true, true, false, Key::C)],
            Action::NewFolder => vec![KeyChord::new(true, true, false, Key::N)],
            Action::Rename => vec![KeyChord::key(Key::F2)],
            Action::Properties => vec![KeyChord::alt(Key::Enter)],
            Action::OpenSelected => vec![KeyChord::key(Key::Enter)],
            Action::Delete => vec![KeyChord::key(Key::Delete)],
            Action::PermanentDelete => vec![KeyChord::ctrl(Key::Delete)],
            Action::SelectAll => vec![KeyChord::ctrl(Key::A)],
            Action::Refresh => vec![KeyChord::ctrl(Key::R), KeyChord::key(Key::F5)],
            Action::AddressBar => vec![KeyChord::alt(Key::D)],
            Action::NavigateBack => {
                vec![KeyChord::alt(Key::ArrowLeft), KeyChord::key(Key::Backspace)]
            }
            Action::NavigateForward => vec![KeyChord::alt(Key::ArrowRight)],
            Action::NavigateUp => vec![KeyChord::alt(Key::ArrowUp)],
            Action::SelectFirst => vec![KeyChord::key(Key::Home)],
            Action::SelectLast => vec![KeyChord::key(Key::End)],
        }
    }
}

/// User overrides as stored in `settings.json`: action name to chord strings.
/// An empty list means the action is unbound.
pub type Overrides = BTreeMap<String, Vec<String>>;

#[derive(Clone, Debug)]
pub struct KeyMap {
    bindings: HashMap<Action, Vec<KeyChord>>,
}

impl Default for KeyMap {
    fn default() -> Self {
        Self::from_overrides(&Overrides::new())
    }
}

impl KeyMap {
    /// Builds the map from stored overrides. Unknown action names are ignored,
    /// and an override whose chords are all unparsable falls back to defaults.
    pub fn from_overrides(overrides: &Overrides) -> Self {
        let bindings = Action::all()
            .map(|action| {
                let chords = overrides
                    .get(action.name())
                    .and_then(|list| {
                        let parsed: Vec<KeyChord> =
                            list.iter().filter_map(|s| s.parse().ok()).collect();
                        (list.is_empty() || !parsed.is_empty()).then_some(parsed)
                    })
                    .unwrap_or_else(|| action.defaults());
                (action, chords)
            })
            .collect();
        Self { bindings }
    }

    pub fn chords(&self, action: Action) -> &[KeyChord] {
        self.bindings.get(&action).map_or(&[], Vec::as_slice)
    }

    /// The action currently bound to `chord`, if any.
    pub fn action_for(&self, chord: KeyChord) -> Option<Action> {
        Action::all().find(|a| self.chords(*a).contains(&chord))
    }

    pub fn set_chords(&mut self, action: Action, chords: Vec<KeyChord>) {
        self.bindings.insert(action, chords);
    }

    /// Only the actions that differ from their defaults.
    pub fn to_overrides(&self) -> Overrides {
        Action::all()
            .filter(|a| self.chords(*a) != a.defaults().as_slice())
            .map(|a| {
                (
                    a.name().to_string(),
                    self.chords(a).iter().map(KeyChord::to_string).collect(),
                )
            })
            .collect()
    }
}

/// The overrides the active map was built from, and the map itself.
static ACTIVE: RwLock<Option<(Overrides, KeyMap)>> = RwLock::new(None);

/// Makes the process-wide map match `overrides`. Cheap when nothing changed.
pub fn sync(overrides: &Overrides) {
    let same = ACTIVE
        .read()
        .ok()
        .is_some_and(|g| g.as_ref().is_some_and(|(o, _)| o == overrides));
    if !same && let Ok(mut guard) = ACTIVE.write() {
        *guard = Some((overrides.clone(), KeyMap::from_overrides(overrides)));
    }
}

fn with_map<R>(f: impl FnOnce(&KeyMap) -> R) -> R {
    if let Ok(guard) = ACTIVE.read()
        && let Some((_, map)) = guard.as_ref()
    {
        return f(map);
    }
    f(&KeyMap::default())
}

/// True when any chord bound to `action` was pressed this frame.
pub fn pressed(input: &egui::InputState, action: Action) -> bool {
    with_map(|map| map.chords(action).iter().any(|c| c.matches(input)))
}

/// Like `pressed`, but also removes the key event so no other widget sees it.
pub fn consume(input: &mut egui::InputState, action: Action) -> bool {
    let chords: Vec<KeyChord> = with_map(|map| map.chords(action).to_vec());
    chords.iter().fold(false, |hit, c| {
        input.consume_key(c.modifiers(), c.key) | hit
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chord_strings_round_trip_for_every_key() {
        for key in egui::Key::ALL {
            for (ctrl, shift, alt) in [
                (false, false, false),
                (true, true, true),
                (true, false, true),
            ] {
                let chord = KeyChord::new(ctrl, shift, alt, *key);
                assert_eq!(chord.to_string().parse::<KeyChord>(), Ok(chord));
            }
        }
    }

    #[test]
    fn chord_display_is_readable() {
        assert_eq!(
            KeyChord::new(true, true, false, egui::Key::N).to_string(),
            "Ctrl+Shift+N"
        );
        assert_eq!(
            "ctrl+shift+n".parse(),
            Ok(KeyChord::new(true, true, false, egui::Key::N))
        );
        assert!("Ctrl+".parse::<KeyChord>().is_err());
        assert!("Hyper+N".parse::<KeyChord>().is_err());
    }

    #[test]
    fn defaults_have_no_duplicate_chords() {
        let mut seen = Vec::new();
        for action in Action::all() {
            for chord in action.defaults() {
                assert!(!seen.contains(&chord), "{chord} bound twice");
                seen.push(chord);
            }
        }
    }

    #[test]
    fn every_action_is_in_exactly_one_group() {
        let all: Vec<Action> = Action::all().collect();
        let mut names: Vec<&str> = all.iter().map(|a| a.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), all.len());
        for action in &all {
            assert_eq!(Action::from_name(action.name()), Some(*action));
        }
    }

    #[test]
    fn empty_overrides_give_defaults_and_serialize_to_nothing() {
        let map = KeyMap::default();
        assert_eq!(map.chords(Action::Refresh).len(), 2);
        assert!(map.to_overrides().is_empty());
    }

    #[test]
    fn overrides_round_trip_through_json() {
        let mut map = KeyMap::default();
        map.set_chords(Action::NewFolder, vec![KeyChord::ctrl(egui::Key::J)]);
        map.set_chords(Action::Rename, Vec::new());
        let overrides = map.to_overrides();
        let json = serde_json::to_string(&overrides).unwrap();
        assert!(json.contains(r#""NewFolder":["Ctrl+J"]"#));
        let back: Overrides = serde_json::from_str(&json).unwrap();
        let rebuilt = KeyMap::from_overrides(&back);
        assert_eq!(
            rebuilt.chords(Action::NewFolder),
            &[KeyChord::ctrl(egui::Key::J)]
        );
        assert!(rebuilt.chords(Action::Rename).is_empty());
        assert_eq!(rebuilt.chords(Action::NewTab), Action::NewTab.defaults());
    }

    #[test]
    fn unknown_actions_and_bad_chords_are_tolerated() {
        let mut overrides = Overrides::new();
        overrides.insert("Teleport".into(), vec!["Ctrl+Q".into()]);
        overrides.insert("NewTab".into(), vec!["Bogus+".into()]);
        overrides.insert("CloseTab".into(), vec!["Ctrl+Y".into(), "nonsense".into()]);
        let map = KeyMap::from_overrides(&overrides);
        assert_eq!(map.chords(Action::NewTab), Action::NewTab.defaults());
        assert_eq!(
            map.chords(Action::CloseTab),
            &[KeyChord::ctrl(egui::Key::Y)]
        );
        assert_eq!(map.action_for(KeyChord::ctrl(egui::Key::Q)), None);
    }
}
