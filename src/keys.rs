use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::api::Feed;
use crate::app::{App, Message, View};

pub fn handle_key(key: KeyEvent, app: &App) -> Option<Message> {
    match key.code {
        KeyCode::Char('q') => return Some(Message::Quit),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return Some(Message::Quit);
        }
        _ => {}
    }

    match app.view {
        View::Stories => handle_stories_key(key),
        View::Comments { .. } => handle_comments_key(key),
    }
}

fn handle_stories_key(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Message::SelectNext),
        KeyCode::Char('k') | KeyCode::Up => Some(Message::SelectPrev),
        KeyCode::Char('g') => Some(Message::SelectFirst),
        KeyCode::Char('G') => Some(Message::SelectLast),
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Message::PageDown)
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Message::PageUp)
        }
        KeyCode::Char('o') => Some(Message::OpenUrl),
        KeyCode::Char('l') | KeyCode::Enter => Some(Message::OpenComments),
        KeyCode::Char('c') => Some(Message::OpenCommentsUrl),
        KeyCode::Char('r') | KeyCode::Char('R') => Some(Message::Refresh),
        KeyCode::Char('H') => Some(Message::PrevFeed),
        KeyCode::Char('L') => Some(Message::NextFeed),
        KeyCode::Char('1') => Some(Message::SwitchFeed(Feed::Top)),
        KeyCode::Char('2') => Some(Message::SwitchFeed(Feed::New)),
        KeyCode::Char('3') => Some(Message::SwitchFeed(Feed::Best)),
        KeyCode::Char('4') => Some(Message::SwitchFeed(Feed::Ask)),
        KeyCode::Char('5') => Some(Message::SwitchFeed(Feed::Show)),
        KeyCode::Char('6') => Some(Message::SwitchFeed(Feed::Jobs)),
        KeyCode::Char('?') => Some(Message::ToggleHelp),
        _ => None,
    }
}

fn handle_comments_key(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Message::SelectNext),
        KeyCode::Char('k') | KeyCode::Up => Some(Message::SelectPrev),
        KeyCode::Char('g') => Some(Message::SelectFirst),
        KeyCode::Char('G') => Some(Message::SelectLast),
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Message::PageDown)
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Message::PageUp)
        }
        KeyCode::Char('o') => Some(Message::OpenUrl),
        KeyCode::Char('c') => Some(Message::OpenCommentsUrl),
        KeyCode::Char('l') => Some(Message::ExpandComment),
        KeyCode::Char('h') => Some(Message::CollapseComment),
        KeyCode::Esc => Some(Message::Back),
        KeyCode::Char('r') | KeyCode::Char('R') => Some(Message::Refresh),
        KeyCode::Char('?') => Some(Message::ToggleHelp),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{ThemeVariant, default_for_variant};
    use crossterm::event::KeyEventState;

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    fn test_app() -> App {
        App::new(default_for_variant(ThemeVariant::Dark))
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
}
