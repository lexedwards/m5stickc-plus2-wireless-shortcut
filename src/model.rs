pub const SHORTCUTS: [Shortcut; 2] = [
    Shortcut {
        label: "Lock Screen",
        combination: "Ctrl+Cmd+Q",
        modifiers: modifier::LEFT_CONTROL | modifier::LEFT_GUI,
        keycode: keycode::Q,
    },
    Shortcut {
        label: "Dictate",
        combination: "Shift+Ctrl+Alt+Cmd+D",
        modifiers: modifier::LEFT_SHIFT
            | modifier::LEFT_CONTROL
            | modifier::LEFT_ALT
            | modifier::LEFT_GUI,
        keycode: keycode::D,
    },
];

mod modifier {
    pub const LEFT_CONTROL: u8 = 0x01;
    pub const LEFT_SHIFT: u8 = 0x02;
    pub const LEFT_ALT: u8 = 0x04;
    pub const LEFT_GUI: u8 = 0x08;
}

mod keycode {
    pub const D: u8 = 0x07;
    pub const Q: u8 = 0x14;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Shortcut {
    pub label: &'static str,
    pub combination: &'static str,
    modifiers: u8,
    keycode: u8,
}

impl Shortcut {
    pub const fn press_report(self) -> [u8; 8] {
        [self.modifiers, 0, self.keycode, 0, 0, 0, 0, 0]
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Screen {
    #[default]
    Send,
    SelectShortcut,
    SelectDevice,
}

impl Screen {
    pub const fn next(self) -> Self {
        match self {
            Self::Send => Self::SelectShortcut,
            Self::SelectShortcut => Self::SelectDevice,
            Self::SelectDevice => Self::Send,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Target {
    #[default]
    All,
    Device1,
    Device2,
}

impl Target {
    pub const fn next(self) -> Self {
        match self {
            Self::All => Self::Device1,
            Self::Device1 => Self::Device2,
            Self::Device2 => Self::All,
        }
    }

    pub const fn storage_value(self) -> u8 {
        match self {
            Self::All => 0,
            Self::Device1 => 1,
            Self::Device2 => 2,
        }
    }

    pub const fn from_storage(value: u8) -> Self {
        match value {
            1 => Self::Device1,
            2 => Self::Device2,
            _ => Self::All,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimaryAction {
    Send { target: Target, shortcut: Shortcut },
    SelectionChanged,
}

#[derive(Debug)]
pub struct AppState {
    pub screen: Screen,
    pub target: Target,
    shortcut_index: usize,
}

impl AppState {
    pub const fn new(target: Target) -> Self {
        Self {
            screen: Screen::Send,
            target,
            shortcut_index: 0,
        }
    }

    pub const fn shortcut(&self) -> Shortcut {
        SHORTCUTS[self.shortcut_index]
    }

    pub fn next_screen(&mut self) {
        self.screen = self.screen.next();
    }

    pub fn primary_action(&mut self) -> PrimaryAction {
        match self.screen {
            Screen::Send => PrimaryAction::Send {
                target: self.target,
                shortcut: self.shortcut(),
            },
            Screen::SelectShortcut => {
                self.shortcut_index = (self.shortcut_index + 1) % SHORTCUTS.len();
                PrimaryAction::SelectionChanged
            }
            Screen::SelectDevice => {
                self.target = self.target.next();
                PrimaryAction::SelectionChanged
            }
        }
    }
}

pub struct DebouncedButton {
    stable: bool,
    candidate: bool,
    consecutive_samples: u8,
}

impl DebouncedButton {
    pub const fn new() -> Self {
        Self {
            stable: false,
            candidate: false,
            consecutive_samples: 0,
        }
    }

    pub fn update(&mut self, pressed: bool) -> bool {
        if pressed != self.candidate {
            self.candidate = pressed;
            self.consecutive_samples = 1;
            return false;
        }

        if self.consecutive_samples < 3 {
            self.consecutive_samples += 1;
        }

        if self.consecutive_samples == 3 && self.stable != self.candidate {
            self.stable = self.candidate;
            return self.stable;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_generate_expected_hid_reports() {
        assert_eq!(SHORTCUTS[0].press_report(), [0x09, 0, 0x14, 0, 0, 0, 0, 0]);
        assert_eq!(SHORTCUTS[1].press_report(), [0x0f, 0, 0x07, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn secondary_button_cycles_all_screens() {
        let mut state = AppState::new(Target::All);

        state.next_screen();
        assert_eq!(state.screen, Screen::SelectShortcut);
        state.next_screen();
        assert_eq!(state.screen, Screen::SelectDevice);
        state.next_screen();
        assert_eq!(state.screen, Screen::Send);
    }

    #[test]
    fn primary_button_changes_only_the_selection_on_selection_screens() {
        let mut state = AppState::new(Target::All);

        state.next_screen();
        assert_eq!(state.primary_action(), PrimaryAction::SelectionChanged);
        assert_eq!(state.shortcut(), SHORTCUTS[1]);

        state.next_screen();
        assert_eq!(state.primary_action(), PrimaryAction::SelectionChanged);
        assert_eq!(state.target, Target::Device1);
    }

    #[test]
    fn send_action_contains_current_shortcut_and_target() {
        let mut state = AppState::new(Target::Device2);

        assert_eq!(
            state.primary_action(),
            PrimaryAction::Send {
                target: Target::Device2,
                shortcut: SHORTCUTS[0],
            }
        );
    }

    #[test]
    fn target_selection_wraps() {
        assert_eq!(Target::All.next(), Target::Device1);
        assert_eq!(Target::Device1.next(), Target::Device2);
        assert_eq!(Target::Device2.next(), Target::All);
    }

    #[test]
    fn debounce_ignores_bounce_and_fires_once_per_press() {
        let mut button = DebouncedButton::new();
        let samples = [
            true, false, true, true, true, true, false, true, false, false, false,
        ];
        let events = samples
            .into_iter()
            .filter(|pressed| button.update(*pressed))
            .count();

        assert_eq!(events, 1);

        assert!(!button.update(true));
        assert!(!button.update(true));
        assert!(button.update(true));
    }
}
