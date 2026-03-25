mod clock;
mod render;
mod theme;

pub use theme::ShellTheme;

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use clock::ClockState;
use veila_renderer::avatar::AvatarAsset;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellAction {
    None,
    Submit(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKey {
    Character(char),
    Backspace,
    Enter,
    Escape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ShellStatus {
    Idle,
    Pending,
    Rejected {
        retry_until: Option<Instant>,
        displayed_retry_seconds: Option<u64>,
    },
}

#[derive(Debug, Clone)]
pub struct ShellState {
    secret: String,
    reveal_secret: bool,
    reveal_toggle_hovered: bool,
    reveal_toggle_pressed: bool,
    focused: bool,
    status: ShellStatus,
    clock: ClockState,
    theme: ShellTheme,
    hint_text: String,
    username_text: Option<String>,
    avatar: AvatarAsset,
}

impl Default for ShellState {
    fn default() -> Self {
        Self::new(ShellTheme::default(), None, None, true)
    }
}

impl ShellState {
    pub fn new(
        theme: ShellTheme,
        user_hint: Option<String>,
        avatar_path: Option<PathBuf>,
        show_username: bool,
    ) -> Self {
        Self {
            secret: String::new(),
            reveal_secret: false,
            reveal_toggle_hovered: false,
            reveal_toggle_pressed: false,
            focused: true,
            status: ShellStatus::Idle,
            clock: ClockState::current(),
            theme,
            hint_text: user_hint
                .filter(|hint| !hint.trim().is_empty())
                .unwrap_or_else(|| String::from("Type your password to unlock")),
            username_text: username_text(show_username),
            avatar: load_avatar(avatar_path),
        }
    }

    pub fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub fn apply_theme(
        &mut self,
        theme: ShellTheme,
        user_hint: Option<String>,
        avatar_path: Option<PathBuf>,
        show_username: bool,
    ) {
        self.theme = theme;
        self.hint_text = user_hint
            .filter(|hint| !hint.trim().is_empty())
            .unwrap_or_else(|| String::from("Type your password to unlock"));
        self.username_text = username_text(show_username);
        self.avatar = load_avatar(avatar_path);
    }

    pub fn handle_key(&mut self, key: ShellKey) -> ShellAction {
        match key {
            ShellKey::Character(character) => {
                if !character.is_control() && self.secret.chars().count() < 128 {
                    self.secret.push(character);
                    self.status = ShellStatus::Idle;
                }
                ShellAction::None
            }
            ShellKey::Backspace => {
                self.secret.pop();
                self.status = ShellStatus::Idle;
                ShellAction::None
            }
            ShellKey::Escape => {
                self.secret.clear();
                self.reveal_secret = false;
                self.reveal_toggle_pressed = false;
                self.status = ShellStatus::Idle;
                ShellAction::None
            }
            ShellKey::Enter => {
                if self.secret.is_empty() {
                    ShellAction::None
                } else {
                    self.status = ShellStatus::Pending;
                    ShellAction::Submit(self.secret.clone())
                }
            }
        }
    }

    pub fn authentication_busy(&mut self) {
        self.status = ShellStatus::Idle;
    }

    pub fn authentication_rejected(&mut self, retry_after_ms: Option<u64>) {
        self.secret.clear();
        self.reveal_secret = false;
        self.reveal_toggle_pressed = false;
        let retry_until = retry_after_ms
            .filter(|retry_after_ms| *retry_after_ms > 0)
            .map(|retry_after_ms| Instant::now() + Duration::from_millis(retry_after_ms));
        let displayed_retry_seconds = retry_until.and_then(current_retry_seconds);
        self.status = ShellStatus::Rejected {
            retry_until,
            displayed_retry_seconds,
        };
    }

    pub fn advance_animated_state(&mut self) -> bool {
        let clock_changed = self.clock.refresh();
        let ShellStatus::Rejected {
            retry_until,
            displayed_retry_seconds,
        } = &mut self.status
        else {
            return clock_changed;
        };

        let next_display = retry_until.and_then(current_retry_seconds);
        if *displayed_retry_seconds == next_display {
            return clock_changed;
        }

        *displayed_retry_seconds = next_display;
        if next_display.is_none() {
            *retry_until = None;
        }

        true
    }

    pub fn handle_pointer_motion(
        &mut self,
        frame_width: i32,
        frame_height: i32,
        x: f64,
        y: f64,
    ) -> bool {
        let x = x.floor() as i32;
        let y = y.floor() as i32;
        let toggle_rect = self.reveal_toggle_rect_for_frame(frame_width, frame_height);
        let hovered = toggle_rect.contains(x, y);
        let changed = self.reveal_toggle_hovered != hovered;
        self.reveal_toggle_hovered = hovered;
        if !hovered && self.reveal_toggle_pressed {
            self.reveal_toggle_pressed = false;
            return true;
        }

        changed
    }

    pub fn handle_pointer_leave(&mut self) -> bool {
        let changed = self.reveal_toggle_hovered || self.reveal_toggle_pressed;
        self.reveal_toggle_hovered = false;
        self.reveal_toggle_pressed = false;
        changed
    }

    pub fn handle_pointer_press(
        &mut self,
        frame_width: i32,
        frame_height: i32,
        x: f64,
        y: f64,
    ) -> bool {
        let x = x.floor() as i32;
        let y = y.floor() as i32;
        let toggle_rect = self.reveal_toggle_rect_for_frame(frame_width, frame_height);
        let pressed = toggle_rect.contains(x, y);
        let changed =
            self.reveal_toggle_pressed != pressed || self.reveal_toggle_hovered != pressed;
        self.reveal_toggle_hovered = pressed;
        self.reveal_toggle_pressed = pressed;
        changed
    }

    pub fn handle_pointer_release(
        &mut self,
        frame_width: i32,
        frame_height: i32,
        x: f64,
        y: f64,
    ) -> bool {
        let x = x.floor() as i32;
        let y = y.floor() as i32;
        let toggle_rect = self.reveal_toggle_rect_for_frame(frame_width, frame_height);
        let hovered = toggle_rect.contains(x, y);
        let toggled = self.reveal_toggle_pressed && hovered;
        let changed =
            self.reveal_toggle_pressed || self.reveal_toggle_hovered != hovered || toggled;
        self.reveal_toggle_pressed = false;
        self.reveal_toggle_hovered = hovered;
        if toggled {
            self.reveal_secret = !self.reveal_secret;
        }

        changed
    }
}

fn load_avatar(avatar_path: Option<PathBuf>) -> AvatarAsset {
    for path in avatar_candidates(avatar_path) {
        match AvatarAsset::load(&path) {
            Ok(avatar) => return avatar,
            Err(error) => {
                tracing::warn!(path = %path.display(), "failed to load avatar image: {error}")
            }
        }
    }

    AvatarAsset::placeholder()
}

fn avatar_candidates(explicit: Option<PathBuf>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(path) = explicit {
        candidates.push(path);
    }

    if let Some(face_path) = default_face_path()
        && !candidates.iter().any(|path| path == &face_path)
    {
        candidates.push(face_path);
    }

    candidates
}

fn default_face_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let path = Path::new(&home).join(".face");
    path.is_file().then_some(path)
}

fn current_retry_seconds(retry_until: Instant) -> Option<u64> {
    let seconds = retry_until
        .saturating_duration_since(Instant::now())
        .as_millis()
        .div_ceil(1_000) as u64;

    if seconds == 0 { None } else { Some(seconds) }
}

fn username_text(show_username: bool) -> Option<String> {
    if !show_username {
        return None;
    }

    std::env::var("USER")
        .ok()
        .or_else(|| std::env::var("LOGNAME").ok())
        .map(|username| username.trim().to_string())
        .filter(|username| !username.is_empty())
}

#[cfg(test)]
mod tests {
    use std::{
        thread,
        time::{Duration, Instant},
    };

    use veila_renderer::{FrameSize, SoftwareBuffer};

    use super::{ShellAction, ShellKey, ShellState, ShellStatus};

    #[test]
    fn edits_and_submits_password_text() {
        let mut shell = ShellState::default();

        assert_eq!(
            shell.handle_key(ShellKey::Character('a')),
            ShellAction::None
        );
        assert_eq!(
            shell.handle_key(ShellKey::Character('b')),
            ShellAction::None
        );
        assert_eq!(
            shell.handle_key(ShellKey::Enter),
            ShellAction::Submit(String::from("ab"))
        );
        assert_eq!(shell.handle_key(ShellKey::Backspace), ShellAction::None);
        assert_eq!(
            shell.handle_key(ShellKey::Enter),
            ShellAction::Submit(String::from("a"))
        );
    }

    #[test]
    fn rejection_clears_secret() {
        let mut shell = ShellState::default();
        shell.handle_key(ShellKey::Character('a'));
        shell.authentication_rejected(Some(1_000));

        assert_eq!(shell.handle_key(ShellKey::Enter), ShellAction::None);
    }

    #[test]
    fn countdown_state_advances_after_timeout() {
        let mut shell = ShellState {
            status: ShellStatus::Rejected {
                retry_until: Some(Instant::now() + Duration::from_millis(1_100)),
                displayed_retry_seconds: Some(2),
            },
            ..ShellState::default()
        };
        thread::sleep(Duration::from_millis(250));

        assert!(shell.advance_animated_state());
    }

    #[test]
    fn renders_non_empty_scene() {
        let mut shell = ShellState::default();
        shell.set_focus(true);
        let mut buffer = SoftwareBuffer::new(FrameSize::new(480, 320)).expect("buffer");
        shell.render(&mut buffer);

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }

    #[test]
    fn starts_visually_focused() {
        let shell = ShellState::default();

        assert!(shell.focused);
    }

    #[test]
    fn toggles_password_reveal_when_eye_is_pressed() {
        let mut shell = ShellState::default();
        shell.handle_key(ShellKey::Character('s'));
        let toggle = shell.reveal_toggle_rect_for_frame(1280, 720);

        assert!(shell.handle_pointer_motion(
            1280,
            720,
            (toggle.x + 2) as f64,
            (toggle.y + 2) as f64,
        ));
        assert!(shell.reveal_toggle_hovered);
        assert!(shell.handle_pointer_press(
            1280,
            720,
            (toggle.x + 2) as f64,
            (toggle.y + 2) as f64,
        ));
        assert!(shell.reveal_toggle_pressed);
        assert!(shell.handle_pointer_release(
            1280,
            720,
            (toggle.x + 2) as f64,
            (toggle.y + 2) as f64,
        ));
        assert!(shell.reveal_secret);
    }

    #[test]
    fn clears_hover_state_when_pointer_leaves_toggle() {
        let mut shell = ShellState::default();
        let toggle = shell.reveal_toggle_rect_for_frame(1280, 720);
        shell.handle_pointer_motion(1280, 720, (toggle.x + 2) as f64, (toggle.y + 2) as f64);

        assert!(shell.handle_pointer_leave());
        assert!(!shell.reveal_toggle_hovered);
        assert!(!shell.reveal_toggle_pressed);
    }

    #[test]
    fn can_disable_username_label() {
        let shell = ShellState::new(Default::default(), None, None, false);

        assert!(shell.username_text.is_none());
    }
}
