use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::api::Feed;
use crate::app::{App, Message, View};

/// A declarative keybinding map that can be composed and extended.
#[derive(Clone)]
pub struct Keymap {
    bindings: Vec<(KeyCode, KeyModifiers, Message)>,
}

impl Keymap {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    /// Add a key binding with no modifiers.
    pub fn bind(mut self, code: KeyCode, message: Message) -> Self {
        self.bindings.push((code, KeyModifiers::NONE, message));
        self
    }

    /// Add a key binding with Ctrl modifier.
    pub fn bind_ctrl(mut self, code: KeyCode, message: Message) -> Self {
        self.bindings.push((code, KeyModifiers::CONTROL, message));
        self
    }

    /// Look up a message for a key event.
    /// Later bindings take precedence over earlier ones.
    pub fn get(&self, event: &KeyEvent) -> Option<Message> {
        self.bindings
            .iter()
            .rev()
            .find(|(code, mods, _)| *code == event.code && event.modifiers.contains(*mods))
            .map(|(_, _, msg)| msg.clone())
    }

    /// Extend this keymap with another. The other keymap's bindings take precedence.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn extend(mut self, other: Keymap) -> Self {
        self.bindings.extend(other.bindings);
        self
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self::new()
    }
}

/// Global keybindings that work in all views.
fn global_keymap() -> Keymap {
    Keymap::new()
        .bind(KeyCode::Char('q'), Message::Quit)
        .bind_ctrl(KeyCode::Char('c'), Message::Quit)
        .bind(KeyCode::Char('`'), Message::ToggleDebug)
        .bind(KeyCode::Char('t'), Message::OpenThemePicker)
}

/// Keybindings for the theme picker popup.
fn theme_picker_keymap() -> Keymap {
    Keymap::new()
        .bind(KeyCode::Char('j'), Message::ThemePickerDown)
        .bind(KeyCode::Down, Message::ThemePickerDown)
        .bind(KeyCode::Char('k'), Message::ThemePickerUp)
        .bind(KeyCode::Up, Message::ThemePickerUp)
        .bind(KeyCode::Enter, Message::ConfirmThemePicker)
        .bind(KeyCode::Esc, Message::CloseThemePicker)
        .bind(KeyCode::Char('q'), Message::CloseThemePicker)
}

/// Navigation keybindings shared between stories and comments views.
fn navigation_keymap() -> Keymap {
    Keymap::new()
        .bind(KeyCode::Char('j'), Message::SelectNext)
        .bind(KeyCode::Down, Message::SelectNext)
        .bind(KeyCode::Char('k'), Message::SelectPrev)
        .bind(KeyCode::Up, Message::SelectPrev)
        .bind(KeyCode::Char('g'), Message::SelectFirst)
        .bind(KeyCode::Char('G'), Message::SelectLast)
        .bind_ctrl(KeyCode::Char('d'), Message::PageDown)
        .bind_ctrl(KeyCode::Char('u'), Message::PageUp)
        .bind(KeyCode::Char('o'), Message::OpenUrl)
        .bind(KeyCode::Char('c'), Message::OpenCommentsUrl)
        .bind(KeyCode::Char('r'), Message::Refresh)
        .bind(KeyCode::Char('R'), Message::Refresh)
        .bind(KeyCode::Char('?'), Message::ToggleHelp)
}

/// Stories view keybindings.
fn stories_keymap() -> Keymap {
    navigation_keymap()
        .bind(KeyCode::Char('l'), Message::OpenComments)
        .bind(KeyCode::Enter, Message::OpenComments)
        .bind(KeyCode::Char('H'), Message::PrevFeed)
        .bind(KeyCode::Char('L'), Message::NextFeed)
        .bind(KeyCode::Char('1'), Message::SwitchFeed(Feed::Top))
        .bind(KeyCode::Char('2'), Message::SwitchFeed(Feed::New))
        .bind(KeyCode::Char('3'), Message::SwitchFeed(Feed::Best))
        .bind(KeyCode::Char('4'), Message::SwitchFeed(Feed::Ask))
        .bind(KeyCode::Char('5'), Message::SwitchFeed(Feed::Show))
        .bind(KeyCode::Char('6'), Message::SwitchFeed(Feed::Jobs))
}

/// Comments view keybindings.
fn comments_keymap() -> Keymap {
    navigation_keymap()
        .bind(KeyCode::Char('l'), Message::ExpandComment)
        .bind(KeyCode::Char('h'), Message::CollapseComment)
        .bind(KeyCode::Char('L'), Message::ExpandSubtree)
        .bind(KeyCode::Char('H'), Message::CollapseSubtree)
        .bind(KeyCode::Char('+'), Message::ExpandThread)
        .bind(KeyCode::Char('='), Message::ExpandThread)
        .bind(KeyCode::Char('-'), Message::CollapseThread)
        .bind(KeyCode::Char('_'), Message::CollapseThread)
        .bind(KeyCode::Esc, Message::Back)
}

pub fn handle_key(key: KeyEvent, app: &App) -> Option<Message> {
    // Theme picker takes priority when open
    if app.theme_picker.is_some() {
        return theme_picker_keymap().get(&key);
    }

    // Global keys first
    if let Some(msg) = global_keymap().get(&key) {
        return Some(msg);
    }

    // View-specific keys
    match app.view {
        View::Stories => stories_keymap().get(&key),
        View::Comments { .. } => comments_keymap().get(&key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{ThemeVariant, default_for_variant};
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    fn make_key_with_mods(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    fn test_app() -> App {
        App::new(default_for_variant(ThemeVariant::Dark))
    }

    fn comments_app() -> App {
        let mut app = test_app();
        app.view = View::Comments {
            story_id: 1,
            story_title: "Test".to_string(),
            story_index: 0,
            story_scroll: 0,
        };
        app
    }

    #[test]
    fn test_quit_key() {
        let app = test_app();
        assert!(matches!(
            handle_key(make_key(KeyCode::Char('q')), &app),
            Some(Message::Quit)
        ));
    }

    #[test]
    fn test_ctrl_c_quit() {
        let app = test_app();
        assert!(matches!(
            handle_key(
                make_key_with_mods(KeyCode::Char('c'), KeyModifiers::CONTROL),
                &app
            ),
            Some(Message::Quit)
        ));
    }

    #[test]
    fn test_navigation_keys() {
        let app = test_app();
        assert!(matches!(
            handle_key(make_key(KeyCode::Char('j')), &app),
            Some(Message::SelectNext)
        ));
        assert!(matches!(
            handle_key(make_key(KeyCode::Char('k')), &app),
            Some(Message::SelectPrev)
        ));
        assert!(matches!(
            handle_key(make_key(KeyCode::Char('g')), &app),
            Some(Message::SelectFirst)
        ));
        assert!(matches!(
            handle_key(make_key(KeyCode::Char('G')), &app),
            Some(Message::SelectLast)
        ));
    }

    #[test]
    fn test_page_navigation() {
        let app = test_app();
        assert!(matches!(
            handle_key(
                make_key_with_mods(KeyCode::Char('d'), KeyModifiers::CONTROL),
                &app
            ),
            Some(Message::PageDown)
        ));
        assert!(matches!(
            handle_key(
                make_key_with_mods(KeyCode::Char('u'), KeyModifiers::CONTROL),
                &app
            ),
            Some(Message::PageUp)
        ));
    }

    #[test]
    fn test_feed_switch_keys() {
        let app = test_app();
        assert!(matches!(
            handle_key(make_key(KeyCode::Char('1')), &app),
            Some(Message::SwitchFeed(Feed::Top))
        ));
        assert!(matches!(
            handle_key(make_key(KeyCode::Char('4')), &app),
            Some(Message::SwitchFeed(Feed::Ask))
        ));
    }

    #[test]
    fn test_feed_cycle_keys() {
        let app = test_app();
        assert!(matches!(
            handle_key(make_key(KeyCode::Char('H')), &app),
            Some(Message::PrevFeed)
        ));
        assert!(matches!(
            handle_key(make_key(KeyCode::Char('L')), &app),
            Some(Message::NextFeed)
        ));
    }

    #[test]
    fn test_comments_expand_collapse() {
        let app = comments_app();
        assert!(matches!(
            handle_key(make_key(KeyCode::Char('l')), &app),
            Some(Message::ExpandComment)
        ));
        assert!(matches!(
            handle_key(make_key(KeyCode::Char('h')), &app),
            Some(Message::CollapseComment)
        ));
        assert!(matches!(
            handle_key(make_key(KeyCode::Char('L')), &app),
            Some(Message::ExpandSubtree)
        ));
        assert!(matches!(
            handle_key(make_key(KeyCode::Char('H')), &app),
            Some(Message::CollapseSubtree)
        ));
    }

    #[test]
    fn test_comments_thread_keys() {
        let app = comments_app();
        assert!(matches!(
            handle_key(make_key(KeyCode::Char('+')), &app),
            Some(Message::ExpandThread)
        ));
        assert!(matches!(
            handle_key(make_key(KeyCode::Char('-')), &app),
            Some(Message::CollapseThread)
        ));
    }

    #[test]
    fn test_comments_back() {
        let app = comments_app();
        assert!(matches!(
            handle_key(make_key(KeyCode::Esc), &app),
            Some(Message::Back)
        ));
    }

    #[test]
    fn test_shared_keys_work_in_both_views() {
        let stories_app = test_app();
        let comments_app = comments_app();

        // Navigation works in both
        assert!(matches!(
            handle_key(make_key(KeyCode::Char('j')), &stories_app),
            Some(Message::SelectNext)
        ));
        assert!(matches!(
            handle_key(make_key(KeyCode::Char('j')), &comments_app),
            Some(Message::SelectNext)
        ));

        // Refresh works in both
        assert!(matches!(
            handle_key(make_key(KeyCode::Char('r')), &stories_app),
            Some(Message::Refresh)
        ));
        assert!(matches!(
            handle_key(make_key(KeyCode::Char('r')), &comments_app),
            Some(Message::Refresh)
        ));
    }

    #[test]
    fn test_keymap_extend_precedence() {
        // Later bindings take precedence
        let base = Keymap::new().bind(KeyCode::Char('x'), Message::Quit);
        let extended = base.extend(Keymap::new().bind(KeyCode::Char('x'), Message::Refresh));

        let event = make_key(KeyCode::Char('x'));
        assert!(matches!(extended.get(&event), Some(Message::Refresh)));
    }

    #[test]
    fn test_unknown_key_returns_none() {
        let app = test_app();
        assert!(handle_key(make_key(KeyCode::F(12)), &app).is_none());
    }
}
