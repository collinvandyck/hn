# hn

A terminal UI for Hacker News built with Rust and Ratatui.

## Development Setup

This project uses `mise` to manage tools. Run `mise install` to set up:

- `rust` (stable for building, nightly for formatting)
- `just` (command runner)
- `gh` (GitHub CLI)
- `lazygit` (git TUI)

To use the tools, you should call `mise exec -- [cargo|just|...] [args...]`

## Quick Reference

Defined in `.justfile`:

```bash
just              # Run TUI in dev mode
just test         # Run tests
just fmt          # Format with nightly rustfmt
just snap         # Update snapshots
just ci           # Full CI check (fmt, lint, test)
just themes       # List available themes
```

## Workflow

After making changes:

1. `just fmt` - Format code (uses nightly rustfmt)
2. `just test` - Run unit tests
3. `just lint` - Check for warnings

## Architecture

**Elm Architecture pattern:**

- `App` struct holds all state (`src/app.rs`)
- `Message` enum defines all state transitions
- `update()` processes messages, `render()` draws UI
- Views are pure functions: `render(frame, app, area)`

**Key modules:**

- `api/` - HN API client with caching, types (Story, Comment, Feed)
- `app.rs` - Application state, message handling, navigation
- `views/` - UI rendering (stories, comments, spinner)
- `theme/` - 12 built-in themes, TOML custom themes, terminal dark/light detection
- `cli.rs` - Clap-based CLI with theme subcommands
- `keys.rs` - Vim-style keybindings
- `test_utils.rs` - Test data builders for view testing

## Features

- Vim keybindings (j/k, g/G, H/L, etc.)
- 6 feeds: Top, New, Best, Ask, Show, Jobs
- Collapsible comment trees with depth coloring
- 12 built-in themes + custom TOML themes
- Auto dark/light detection
- Loading spinner animation

## Roadmap

ROADMAP.md contains possible features that may be started on. It should not be
used for normal project planning and only should be updated when specifically
asked.

## Testing

View tests use `ratatui::TestBackend` + `insta` snapshots:

```rust
let app = TestAppBuilder::new()
.with_stories(sample_stories())
.selected(0)
.build();

let output = render_to_string(80, 24, | frame| {
views::stories::render(frame, & app, frame.area());
});

insta::assert_snapshot!(output);
```

Snapshots live in `src/views/snapshots/` and are version controlled.

**Test data builders:** `StoryBuilder`, `CommentBuilder`, `TestAppBuilder` in `test_utils.rs`

**UI changes require snapshot updates:** When modifying view rendering logic, add or update snapshot tests to cover the
new behavior. Run `just test` to see failures, then `just snap` to update. Review the diff to verify the rendered output
matches expectations.

## CLI

```bash
hn                        # Run TUI
hn --theme monokai        # Use specific theme
hn --dark / --light       # Force variant
hn theme list             # List themes
hn theme show <name>      # Print theme TOML
hn theme path             # Custom themes directory
```

## Theme System

Themes define semantic colors (story_title, story_score, comment_depth_colors, etc.). See `src/theme/builtin.rs` for
examples. Custom themes go in `~/.config/hn/themes/*.toml`.

## Writing Style

User-facing prose (README, help text, CLI output) should be lowercase and wry. Understated rather than enthusiastic. Dry
humor lives in small word choices, not jokes. Don't try too hard. State what things do without overselling. See
README.md for the voice.

## Code Style

**Modular code:** Keep functions focused and composable. Prefer small, single-purpose modules over large files.

**Blank Lines:** Avoid adding blank lines in fns/methods unless it greatly enhances readability.

**Bounds checks:** Prefer `idx < len - 1` over `idx + 1 < len` for clarity.

**Tests:** High signal-to-noise ratio. Use builders and helpers (`TestAppBuilder`, `sample_stories()`) to hide setup
boilerplate. Test names should describe the behavior being verified. The test body should make the "what" immediately
clear. Modifications and new features should have high quality test coverage.

**Comments:** Only where they add real value. No comments that merely restate what the code does. Good comments explain
*why* something non-obvious exists, document tricky edge cases, or clarify intent that isn't obvious from the code
itself. If code needs a comment to be understood, first consider if the code can be made clearer.

**Markdown formatting:** Use bold sparingly—only for key terms on first use or critical warnings, not for emphasis in
running text. Avoid emojis entirely. Let the content speak for itself.

**Markdown tables:** Format tables for terminal readability. Align columns by padding cells to consistent widths.
Headers and separators should match the column width. Example:

```markdown
| #  | Feature    | Rationale                                 |
|----|------------|-------------------------------------------|
| 1  | Pagination | API supports it, UI doesn't expose it.    |
| 2  | Bookmarks  | Completes core reading workflow.          |
```
