//! Dynamic help text generation from keymaps.

use crate::app::Message;
use crate::keys::{Keymap, format_key};

/// A single help item representing one or more related actions.
pub struct HelpItem {
    /// Messages to look up keys for. Keys are joined with "/".
    messages: Vec<Message>,
    /// The label to show (e.g., "nav", "expand", "quit").
    label: &'static str,
}

impl HelpItem {
    /// Create a help item for a single action.
    pub fn new(message: Message, label: &'static str) -> Self {
        Self {
            messages: vec![message],
            label,
        }
    }

    /// Create a help item for paired actions (e.g., j/k for next/prev).
    pub fn pair(first: Message, second: Message, label: &'static str) -> Self {
        Self {
            messages: vec![first, second],
            label,
        }
    }

    /// Format this help item using the given keymap.
    /// Returns None if no keys are bound for any of the messages.
    pub fn format(&self, keymap: &Keymap) -> Option<String> {
        let keys: Vec<String> = self
            .messages
            .iter()
            .filter_map(|msg| {
                keymap
                    .find_key(msg)
                    .map(|(code, mods)| format_key(code, mods))
            })
            .collect();
        if keys.is_empty() {
            return None;
        }
        Some(format!("{}:{}", keys.join("/"), self.label))
    }

    /// Format this help item for overlay display.
    /// Returns (`keys_string`, label) or None if no keys are bound.
    pub fn format_for_overlay(&self, keymap: &Keymap) -> Option<(String, &'static str)> {
        let keys: Vec<String> = self
            .messages
            .iter()
            .filter_map(|msg| {
                keymap
                    .find_key(msg)
                    .map(|(code, mods)| format_key(code, mods))
            })
            .collect();
        if keys.is_empty() {
            return None;
        }
        Some((keys.join("/"), self.label))
    }
}

/// A collection of help items for a specific context.
pub struct HelpConfig {
    /// Items to show in expanded (full help) mode.
    pub expanded: Vec<HelpItem>,
    /// Items to show in compact (minimal) mode.
    pub compact: Vec<HelpItem>,
}

impl HelpConfig {
    /// Format help text for the given mode.
    pub fn format(&self, keymap: &Keymap, show_expanded: bool) -> String {
        let items = if show_expanded {
            &self.expanded
        } else {
            &self.compact
        };
        items
            .iter()
            .filter_map(|item| item.format(keymap))
            .collect::<Vec<_>>()
            .join("  ")
    }
}

/// Help configuration for the stories view.
pub fn stories_help() -> HelpConfig {
    use Message::{
        CopyUrl, NextFeed, OpenComments, OpenHnPage, OpenThemePicker, OpenUrl, PrevFeed, Quit,
        Refresh, SelectFirst, SelectLast, SelectNext, SelectPrev, ToggleDebug, ToggleFavorite,
        ToggleHelp,
    };
    HelpConfig {
        expanded: vec![
            HelpItem::pair(SelectNext, SelectPrev, "nav"),
            HelpItem::pair(SelectFirst, SelectLast, "top/bottom"),
            HelpItem::pair(PrevFeed, NextFeed, "feeds"),
            HelpItem::new(OpenUrl, "open"),
            HelpItem::new(OpenHnPage, "open on hn"),
            HelpItem::new(CopyUrl, "copy"),
            HelpItem::new(OpenComments, "comments"),
            HelpItem::new(ToggleFavorite, "fav"),
            HelpItem::new(Refresh, "refresh"),
            HelpItem::new(OpenThemePicker, "themes"),
            HelpItem::new(ToggleDebug, "debug"),
            HelpItem::new(Quit, "quit"),
            HelpItem::new(ToggleHelp, "hide"),
        ],
        compact: vec![
            HelpItem::pair(PrevFeed, NextFeed, "feeds"),
            HelpItem::new(ToggleFavorite, "fav"),
            HelpItem::new(ToggleHelp, "help"),
            HelpItem::new(Quit, "quit"),
        ],
    }
}

/// Help configuration for the comments view.
pub fn comments_help() -> HelpConfig {
    use Message::{
        Back, CollapseComment, CollapseSubtree, CollapseThread, CopyStoryUrl, CopyUrl,
        ExpandComment, ExpandSubtree, ExpandThread, OpenStoryUrl, OpenThemePicker, OpenUrl, Quit,
        Refresh, SelectNext, SelectPrev, ToggleDebug, ToggleFavorite, ToggleHelp,
        ToggleStoryFavorite,
    };
    HelpConfig {
        expanded: vec![
            HelpItem::pair(SelectNext, SelectPrev, "nav"),
            HelpItem::pair(ExpandComment, CollapseComment, "expand"),
            HelpItem::pair(ExpandSubtree, CollapseSubtree, "subtree"),
            HelpItem::pair(ExpandThread, CollapseThread, "thread"),
            HelpItem::new(OpenUrl, "link"),
            HelpItem::new(OpenStoryUrl, "story"),
            HelpItem::new(CopyUrl, "copy"),
            HelpItem::new(CopyStoryUrl, "copy story"),
            HelpItem::new(ToggleFavorite, "fav"),
            HelpItem::new(ToggleStoryFavorite, "fav story"),
            HelpItem::new(Back, "back"),
            HelpItem::new(Refresh, "refresh"),
            HelpItem::new(OpenThemePicker, "themes"),
            HelpItem::new(ToggleDebug, "debug"),
            HelpItem::new(Quit, "quit"),
            HelpItem::new(ToggleHelp, "hide"),
        ],
        compact: vec![
            HelpItem::pair(ExpandComment, CollapseComment, "expand"),
            HelpItem::pair(ExpandSubtree, CollapseSubtree, "subtree"),
            HelpItem::pair(ExpandThread, CollapseThread, "thread"),
            HelpItem::new(ToggleFavorite, "fav"),
            HelpItem::new(Back, "back"),
            HelpItem::new(ToggleHelp, "help"),
        ],
    }
}

/// Help configuration for the theme picker.
pub fn theme_picker_help() -> HelpConfig {
    use Message::{CloseThemePicker, ConfirmThemePicker, ThemePickerDown, ThemePickerUp};
    HelpConfig {
        expanded: vec![
            HelpItem::pair(ThemePickerDown, ThemePickerUp, "select"),
            HelpItem::new(ConfirmThemePicker, "confirm"),
            HelpItem::new(CloseThemePicker, "cancel"),
        ],
        compact: vec![
            HelpItem::pair(ThemePickerDown, ThemePickerUp, "select"),
            HelpItem::new(ConfirmThemePicker, "confirm"),
            HelpItem::new(CloseThemePicker, "cancel"),
        ],
    }
}

/// Help items for the stories view overlay.
pub fn stories_overlay_items() -> Vec<HelpItem> {
    use Message::{
        CopyUrl, NextFeed, OpenComments, OpenHnPage, OpenThemePicker, OpenUrl, PrevFeed, Quit,
        Refresh, SelectFirst, SelectLast, SelectNext, SelectPrev, ToggleDebug, ToggleFavorite,
        ToggleHelp,
    };
    vec![
        HelpItem::pair(SelectNext, SelectPrev, "navigate"),
        HelpItem::pair(SelectFirst, SelectLast, "top/bottom"),
        HelpItem::pair(PrevFeed, NextFeed, "switch feeds"),
        HelpItem::new(OpenComments, "open comments"),
        HelpItem::new(OpenUrl, "open link"),
        HelpItem::new(OpenHnPage, "open on hn"),
        HelpItem::new(CopyUrl, "copy url"),
        HelpItem::new(ToggleFavorite, "favorite"),
        HelpItem::new(Refresh, "refresh"),
        HelpItem::new(OpenThemePicker, "themes"),
        HelpItem::new(ToggleDebug, "debug"),
        HelpItem::new(Quit, "quit"),
        HelpItem::new(ToggleHelp, "close"),
    ]
}

/// Help items for the comments view overlay.
pub fn comments_overlay_items() -> Vec<HelpItem> {
    use Message::{
        Back, CollapseComment, CollapseSubtree, CollapseThread, CopyStoryUrl, CopyUrl,
        ExpandComment, ExpandSubtree, ExpandThread, OpenStoryUrl, OpenThemePicker, OpenUrl, Quit,
        Refresh, SelectNext, SelectPrev, ToggleDebug, ToggleFavorite, ToggleHelp,
        ToggleStoryFavorite,
    };
    vec![
        HelpItem::pair(SelectNext, SelectPrev, "navigate"),
        HelpItem::pair(ExpandComment, CollapseComment, "expand/collapse"),
        HelpItem::pair(ExpandSubtree, CollapseSubtree, "subtree"),
        HelpItem::pair(ExpandThread, CollapseThread, "all comments"),
        HelpItem::new(OpenUrl, "open comment link"),
        HelpItem::new(OpenStoryUrl, "open story link"),
        HelpItem::new(CopyUrl, "copy url"),
        HelpItem::new(CopyStoryUrl, "copy story url"),
        HelpItem::new(ToggleFavorite, "favorite comment"),
        HelpItem::new(ToggleStoryFavorite, "favorite story"),
        HelpItem::new(Back, "back to stories"),
        HelpItem::new(Refresh, "refresh"),
        HelpItem::new(OpenThemePicker, "themes"),
        HelpItem::new(ToggleDebug, "debug"),
        HelpItem::new(Quit, "quit"),
        HelpItem::new(ToggleHelp, "close"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::{comments_keymap, global_keymap, stories_keymap, theme_picker_keymap};

    #[test]
    fn stories_help_expanded_contains_expected_items() {
        let keymap = global_keymap().extend(stories_keymap());
        let help = stories_help().format(&keymap, true);
        assert!(help.contains("j/k:nav"));
        assert!(help.contains("g/G:top/bottom"));
        assert!(help.contains("H/L:feeds"));
        assert!(help.contains("l:comments"));
        assert!(help.contains("q:quit"));
    }

    #[test]
    fn stories_help_compact_is_shorter() {
        let keymap = global_keymap().extend(stories_keymap());
        let expanded = stories_help().format(&keymap, true);
        let compact = stories_help().format(&keymap, false);
        assert!(compact.len() < expanded.len());
        assert!(compact.contains("H/L:feeds"));
        assert!(compact.contains("?:help"));
    }

    #[test]
    fn comments_help_has_comment_specific_keys() {
        let keymap = global_keymap().extend(comments_keymap());
        let help = comments_help().format(&keymap, true);
        assert!(help.contains("l/h:expand"));
        assert!(help.contains("L/H:subtree"));
        assert!(help.contains("+/-:thread"));
        assert!(help.contains("Esc:back"));
    }

    #[test]
    fn theme_picker_help_shows_controls() {
        let keymap = theme_picker_keymap();
        let help = theme_picker_help().format(&keymap, true);
        assert!(help.contains("j/k:select"));
        assert!(help.contains("Enter:confirm"));
        assert!(help.contains("Esc:cancel"));
    }

    #[test]
    fn help_item_returns_none_for_unbound_message() {
        let keymap = Keymap::new(); // Empty keymap
        let item = HelpItem::new(Message::Quit, "quit");
        assert!(item.format(&keymap).is_none());
    }
}
