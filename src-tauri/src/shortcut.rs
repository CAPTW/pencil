use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShortcutCandidate {
    pub(crate) modifiers: Vec<String>,
    pub(crate) key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrimaryShortcut {
    pub(crate) modifiers: Vec<String>,
    pub(crate) key: String,
    pub(crate) display: String,
}

impl Default for PrimaryShortcut {
    fn default() -> Self {
        Self {
            modifiers: vec!["CTRL".to_string(), "SHIFT".to_string()],
            key: "G".to_string(),
            display: "Ctrl+Shift+G".to_string(),
        }
    }
}

impl PrimaryShortcut {
    pub(crate) fn from_candidate(candidate: ShortcutCandidate) -> Result<Self, &'static str> {
        let mut has_ctrl = false;
        let mut has_alt = false;
        let mut has_shift = false;
        let mut has_win = false;

        for modifier in candidate.modifiers {
            match modifier.trim().to_ascii_uppercase().as_str() {
                "CTRL" | "CONTROL" => has_ctrl = true,
                "ALT" => has_alt = true,
                "SHIFT" => has_shift = true,
                "WIN" | "SUPER" | "META" => has_win = true,
                _ => return Err("invalid_shortcut_modifier"),
            }
        }

        if !has_ctrl && !has_alt && !has_shift && !has_win {
            return Err("shortcut_modifier_required");
        }

        let key = normalize_key(&candidate.key)?;
        if (has_alt && key == "F4")
            || (has_win && key == "L")
            || (has_ctrl && has_alt && key == "DELETE")
        {
            return Err("reserved_shortcut");
        }

        let mut modifiers = Vec::new();
        let mut display_parts = Vec::new();
        if has_ctrl {
            modifiers.push("CTRL".to_string());
            display_parts.push("Ctrl".to_string());
        }
        if has_alt {
            modifiers.push("ALT".to_string());
            display_parts.push("Alt".to_string());
        }
        if has_shift {
            modifiers.push("SHIFT".to_string());
            display_parts.push("Shift".to_string());
        }
        if has_win {
            modifiers.push("WIN".to_string());
            display_parts.push("Win".to_string());
        }
        display_parts.push(display_key(&key).to_string());

        Ok(Self {
            modifiers,
            key,
            display: display_parts.join("+"),
        })
    }

    pub(crate) fn registration_string(&self) -> String {
        let mut parts = self
            .modifiers
            .iter()
            .map(|modifier| match modifier.as_str() {
                "CTRL" => "Ctrl",
                "ALT" => "Alt",
                "SHIFT" => "Shift",
                "WIN" => "Super",
                _ => modifier.as_str(),
            })
            .collect::<Vec<_>>();
        parts.push(display_key(&self.key));
        parts.join("+")
    }
}

fn normalize_key(value: &str) -> Result<String, &'static str> {
    let key = value.trim().to_ascii_uppercase();
    let is_letter = key.len() == 1 && key.as_bytes()[0].is_ascii_alphabetic();
    let is_digit = key.len() == 1 && key.as_bytes()[0].is_ascii_digit();
    let is_function = key
        .strip_prefix('F')
        .and_then(|number| number.parse::<u8>().ok())
        .is_some_and(|number| (1..=24).contains(&number));
    let is_named = matches!(
        key.as_str(),
        "BACKSPACE"
            | "DELETE"
            | "END"
            | "ENTER"
            | "ESCAPE"
            | "HOME"
            | "INSERT"
            | "PAGEDOWN"
            | "PAGEUP"
            | "SPACE"
            | "TAB"
            | "ARROWDOWN"
            | "ARROWLEFT"
            | "ARROWRIGHT"
            | "ARROWUP"
    );

    if is_letter || is_digit || is_function || is_named {
        Ok(key)
    } else {
        Err("invalid_shortcut_key")
    }
}

fn display_key(key: &str) -> &str {
    match key {
        "BACKSPACE" => "Backspace",
        "DELETE" => "Delete",
        "END" => "End",
        "ENTER" => "Enter",
        "ESCAPE" => "Escape",
        "HOME" => "Home",
        "INSERT" => "Insert",
        "PAGEDOWN" => "PageDown",
        "PAGEUP" => "PageUp",
        "SPACE" => "Space",
        "TAB" => "Tab",
        "ARROWDOWN" => "ArrowDown",
        "ARROWLEFT" => "ArrowLeft",
        "ARROWRIGHT" => "ArrowRight",
        "ARROWUP" => "ArrowUp",
        _ => key,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ShortcutRegistrarError {
    Conflict,
    Failed,
}

pub(crate) trait ShortcutRegistrar {
    fn register(&mut self, shortcut: &PrimaryShortcut) -> Result<(), ShortcutRegistrarError>;
    fn unregister(&mut self, shortcut: &PrimaryShortcut) -> Result<(), ShortcutRegistrarError>;
}

pub(crate) trait ShortcutPersistence {
    fn persist(&mut self, shortcut: &PrimaryShortcut) -> Result<(), ()>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ShortcutUpdateStatus {
    Applied,
    Unchanged,
    Conflict,
    Invalid,
    PersistenceFailedRolledBack,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ShortcutStartupStatus {
    RegisteredSaved,
    FallbackDefault,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ShortcutEventState {
    Pressed,
    Released,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ShortcutActivation {
    CaptureSelection,
    HideWidget,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WidgetVisibility {
    Hidden,
    Visible,
}

#[derive(Default)]
pub(crate) struct ShortcutTriggerGate {
    pressed: bool,
}

impl ShortcutTriggerGate {
    pub(crate) fn handle(&mut self, is_active: bool, state: ShortcutEventState) -> bool {
        match state {
            ShortcutEventState::Released => {
                self.pressed = false;
                false
            }
            ShortcutEventState::Pressed if !is_active => false,
            ShortcutEventState::Pressed if self.pressed => false,
            ShortcutEventState::Pressed => {
                self.pressed = true;
                true
            }
        }
    }
}

pub(crate) fn route_shortcut_activation(
    gate: &mut ShortcutTriggerGate,
    is_active: bool,
    state: ShortcutEventState,
    widget_visibility: WidgetVisibility,
) -> Option<ShortcutActivation> {
    if !dispatch_shortcut_trigger(gate, is_active, state, || {}) {
        return None;
    }

    Some(if widget_visibility == WidgetVisibility::Visible {
        ShortcutActivation::HideWidget
    } else {
        ShortcutActivation::CaptureSelection
    })
}

pub(crate) fn dispatch_shortcut_trigger(
    gate: &mut ShortcutTriggerGate,
    is_active: bool,
    state: ShortcutEventState,
    on_capture: impl FnOnce(),
) -> bool {
    if !gate.handle(is_active, state) {
        return false;
    }

    on_capture();
    true
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ShortcutUpdateOutcome {
    pub(crate) status: ShortcutUpdateStatus,
}

pub(crate) struct ShortcutManager {
    active: PrimaryShortcut,
}

impl ShortcutManager {
    pub(crate) fn new(active: PrimaryShortcut) -> Self {
        Self { active }
    }

    pub(crate) fn active(&self) -> &PrimaryShortcut {
        &self.active
    }

    pub(crate) fn activate_startup<R: ShortcutRegistrar, P: ShortcutPersistence>(
        &mut self,
        saved: PrimaryShortcut,
        registrar: &mut R,
        persistence: &mut P,
    ) -> ShortcutStartupStatus {
        if registrar.register(&saved).is_ok() {
            self.active = saved;
            return ShortcutStartupStatus::RegisteredSaved;
        }

        let fallback = PrimaryShortcut::default();
        if saved == fallback || registrar.register(&fallback).is_err() {
            return ShortcutStartupStatus::Failed;
        }

        self.active = fallback.clone();
        if persistence.persist(&fallback).is_ok() {
            ShortcutStartupStatus::FallbackDefault
        } else {
            ShortcutStartupStatus::Failed
        }
    }

    pub(crate) fn update<R: ShortcutRegistrar, P: ShortcutPersistence>(
        &mut self,
        candidate: ShortcutCandidate,
        registrar: &mut R,
        persistence: &mut P,
    ) -> ShortcutUpdateOutcome {
        let next = match PrimaryShortcut::from_candidate(candidate) {
            Ok(shortcut) => shortcut,
            Err(_) => {
                return ShortcutUpdateOutcome {
                    status: ShortcutUpdateStatus::Invalid,
                }
            }
        };

        if next == self.active {
            return ShortcutUpdateOutcome {
                status: ShortcutUpdateStatus::Unchanged,
            };
        }

        if let Err(error) = registrar.register(&next) {
            return ShortcutUpdateOutcome {
                status: match error {
                    ShortcutRegistrarError::Conflict => ShortcutUpdateStatus::Conflict,
                    ShortcutRegistrarError::Failed => ShortcutUpdateStatus::Failed,
                },
            };
        }

        if registrar.unregister(&self.active).is_err() {
            let _ = registrar.unregister(&next);
            return ShortcutUpdateOutcome {
                status: ShortcutUpdateStatus::Failed,
            };
        }

        if persistence.persist(&next).is_err() {
            let removed_candidate = registrar.unregister(&next).is_ok();
            let restored_previous = registrar.register(&self.active).is_ok();
            return ShortcutUpdateOutcome {
                status: if removed_candidate && restored_previous {
                    ShortcutUpdateStatus::PersistenceFailedRolledBack
                } else {
                    ShortcutUpdateStatus::Failed
                },
            };
        }

        self.active = next;
        ShortcutUpdateOutcome {
            status: ShortcutUpdateStatus::Applied,
        }
    }
}

impl PrimaryShortcut {
    pub(crate) fn candidate(&self) -> ShortcutCandidate {
        ShortcutCandidate {
            modifiers: self.modifiers.clone(),
            key: self.key.clone(),
        }
    }
}
