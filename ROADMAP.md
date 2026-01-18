# Roadmap

Ideas for future features. The priority list ranks by value, effort, and whether a feature enables others. Detailed
descriptions are in the sections below.

---

## Completed

| Feature    | Notes                                                                        |
|------------|------------------------------------------------------------------------------|
| Pagination | Infinite scroll with automatic loading as you navigate. Background fetching. |
| Debug Mode | Toggle with backtick (`). Shows running tasks and event log.                 |

---

## Priority

| #  | Feature                  | Rationale                                                                     |
|----|--------------------------|-------------------------------------------------------------------------------|
| 1  | SQLite Storage           | Persist API data locally. Foundation for bookmarks, history, offline mode.    |
| 2  | Deep Comment Loading     | Fetch comments beyond max depth on demand. Completes the reading experience.  |
| 3  | Read/Unread Tracking     | Transforms app from viewer to daily-driver. Built on SQLite storage.          |
| 4  | Local Bookmarks          | Completes core reading workflow. Built on SQLite storage.                     |
| 5  | Copy to Clipboard        | Quick win, immediate utility. Users constantly want to share links.           |
| 6  | Comment Enhancements     | Highlight OP, jump between top-level—small effort, better reading experience. |
| 7  | View History             | Natural extension of read tracking. Built on SQLite storage.                  |
| 8  | Search                   | High value for finding old discussions. Algolia API is straightforward.       |
| 9  | User Profiles            | View karma, submissions, comments. Useful context when reading threads.       |
| 10 | Story Filtering          | Hide low-score stories, block domains. Personalization without account.       |
| 11 | Mouse Support            | Events already captured. Click-to-select, scroll wheel. Quick win.            |
| 12 | Status Bar Improvements  | Polish: position indicator, time since refresh, unread count.                 |
| 13 | Background Refresh       | Keep feeds fresh automatically. Nice for leaving app open.                    |
| 14 | Preloading               | Prefetch next page, comments for nearby stories. Snappier feel.               |
| 15 | Code Block Formatting    | Better rendering for technical discussions. Moderate effort.                  |
| 16 | Customizable Keybindings | Power user feature. Config file for remapping keys.                           |
| 17 | Export Thread            | Save discussions as markdown. Useful for reference.                           |
| 18 | Split View               | Stories + comments side-by-side. Ambitious UI change.                         |
| 19 | Link Preview             | Fetch page title/description. Opt-in for privacy. Nice-to-have.               |
| 20 | Offline Mode             | SQLite approach makes this much simpler. Query local DB when offline.         |
| 21 | Login Support            | Cookie-based auth. Enables upvoting/commenting. Security-sensitive.           |
| 22 | Upvoting                 | Requires login. Visual indicator for upvoted items.                           |
| 23 | Commenting & Replies     | Requires login. Compose in $EDITOR. Most complex account feature.             |
| 24 | Share Integration        | Platform-specific (macOS share sheet, etc.). Limited audience.                |
| 25 | Screen Reader Support    | Accessibility: focus announcements, terminal reader compat.                   |
| 26 | High Contrast Mode       | Accessibility: dedicated theme, disable colors option.                        |
| 27 | Plugin System            | Lua/WASM extensibility. Very ambitious, likely overkill.                      |
| 28 | Focus Mode               | Hide scores/counts. Niche but interesting for mindful reading.                |
| 29 | Multi-Account            | Switch HN accounts. Very niche use case.                                      |
| 30 | Comment Threading Viz    | ASCII tree view like `git log --graph`. Fun but niche.                        |
| 31 | HN "Wrapped"             | Reading stats. Fun year-end feature, requires history first.                  |
| 32 | Gemini/Gopher Support    | Text-protocol fetching. Very niche.                                           |

---

## SQLite Storage

Store all fetched API data in a local SQLite database. This provides a foundation for many other features.

### Schema

Mirror the HN API structure:

- `items` table: stories, comments, polls, jobs (all item types)
- `feeds` table: cached feed orderings (top, new, best, etc.)
- `feed_items` junction table: feed membership with position

### Behavior

- Write-through: save items as they're fetched from the API
- Read-through: check local DB before hitting the API
- TTL-based freshness: track `fetched_at`, respect cache expiry
- No blocking: writes happen async, don't slow down the UI

### Benefits

- **Bookmarks**: just a `bookmarked_at` column on items
- **Read tracking**: `read_at` column on items
- **View history**: query items ordered by `last_viewed_at`
- **Offline mode**: serve from local DB when network unavailable
- **Faster restarts**: immediate display of cached content

### Location

`~/.config/hn/data.db` (XDG data directory)

### Migration

rusqlite with embedded migrations. Version table tracks schema changes.

---

## Navigation & Discovery

### Pagination / Infinite Scroll ✓

Implemented with automatic infinite scroll. Stories load in the background as you navigate near the bottom of the list.
The UI remains responsive during fetches with a spinner indicator in the tab bar.

### Search

HN has Algolia-powered search. Could integrate:

- Full-text search across stories and comments
- Filter by date range, points threshold, author
- Search within current comment thread (local, no API)

### User Profiles

View any user's profile by pressing a key on their username:

- Karma, about text, account age
- Recent submissions and comments
- Could cache profiles like we cache items

### Story Filtering

Local filters to customize what you see:

- Hide stories below N points
- Hide stories with certain domains
- Keyword blocklist
- Show only self-posts or link posts

---

## Reading Experience

### Deep Comment Loading

Currently comments are fetched to a max depth of 5 levels. Comments beyond this depth exist but aren't loaded, showing
`[+] N replies` that can't be expanded. This feature would:

- Detect when user tries to expand a comment whose children weren't fetched
- Fetch the next level(s) of comments on demand
- Seamlessly insert them into the existing comment tree
- Show a loading indicator while fetching

Implementation approach:

- Track which comment IDs have unfetched children (kids not in `attempted` set)
- On expand of such a comment, trigger async fetch of its children
- Update the comments list and re-render
- Could fetch one level at a time or batch multiple levels

This removes the artificial depth limit and lets users explore the full depth of any discussion.

### Read/Unread Tracking

Persist which stories you've opened:

- Dim or mark read stories
- "Jump to next unread" keybinding
- Clear read history command
- Built on SQLite storage: `read_at` timestamp column

### Comment Enhancements

- Highlight OP's comments distinctly
- Show comment score (currently hidden in API but available)
- Jump between top-level comments (skip replies)
- Expand/collapse all at once
- "New" badge for comments posted since you last viewed the thread

### Code Block Formatting

Comments often contain code. Could:

- Detect code blocks (indentation or backticks in text)
- Apply monospace styling
- Optional: syntax highlighting via `syntect`

### Link Preview

When cursor is on a story, show preview of the URL:

- Domain, page title, description (via HEAD request or meta scraping)
- Could be a toggleable panel or popup
- Respect user privacy—make it opt-in

---

## Bookmarks & History

### Local Bookmarks

Save stories for later without needing an HN account:

- Press `b` to bookmark current story
- Dedicated "Bookmarks" feed in the tab bar
- Built on SQLite storage: `bookmarked_at` timestamp column
- Export bookmarks to markdown

### View History

Track what you've read:

- "Recently Viewed" feed
- Built on SQLite storage: `last_viewed_at` timestamp column
- Natural extension of read/unread tracking

---

## HN Account Integration

### Login Support

HN doesn't have OAuth, but cookie-based auth could work:

- Login via username/password (stored securely in keyring)
- Or paste session cookie manually

### Upvoting

Once logged in:

- Upvote stories and comments
- Visual indicator of what you've upvoted
- Undo upvote

### Commenting & Replies

- Reply to comments inline
- Compose in `$EDITOR` for longer replies
- Preview before posting
- Submit new stories

Note: These require careful implementation—HN has rate limits and anti-abuse measures.

---

## Performance & Offline

### Background Refresh

Keep feeds fresh without manual refresh:

- Periodic background fetch (configurable interval)
- Only refresh visible/active feed
- Show indicator when new stories available

### Offline Mode

With SQLite storage in place, offline mode becomes straightforward:

- Query local DB when network unavailable
- Show staleness indicator on cached content
- "Save for offline" command to prefetch specific threads
- Sync when back online

### Preloading

Speculatively load content:

- Prefetch comments for stories near cursor
- Prefetch next page of stories
- Balance between responsiveness and bandwidth

---

## UI Polish

### Mouse Support

Crossterm already captures mouse events. Could add:

- Click to select story/comment
- Click tabs to switch feeds
- Scroll wheel navigation
- Click links to open

### Split View

View stories and comments side-by-side:

- Configurable layout (horizontal/vertical split)
- Stories list on left, comments on right
- Update comments as you navigate stories

### Customizable Keybindings

Let users remap keys:

- Config file in `~/.config/hn/keys.toml`
- Support for common schemes (vim, emacs, arrow-only)
- Show current bindings in help overlay

### Status Bar Improvements

- Show time since last refresh
- Network indicator (online/offline)
- Story position in feed (e.g., "15/500")
- Unread count badge

---

## Export & Sharing

### Export Thread

Save a comment thread for reference:

- Export to markdown, plain text, or JSON
- Include metadata (title, URL, author, time)
- Optionally include only expanded comments

### Copy to Clipboard

Quick copy operations:

- Copy story URL
- Copy HN discussion URL
- Copy selected comment text
- Copy story as markdown link

### Share Integration

Platform-specific sharing:

- macOS: Share sheet
- Linux: xdg-open with share URLs
- Generate shareable short links

---

## Developer Experience

### Debug Mode ✓

Implemented with backtick (`) toggle. Shows:

- Running async tasks with elapsed time
- Event log of completed operations
  Future enhancements could add cache stats, API timing details, and log-to-file.

### Plugin System

Allow extending functionality:

- Lua or WASM plugins
- Hooks for events (story selected, comment loaded, etc.)
- Custom commands
- Theme hot-reloading

This is ambitious and probably overkill, but could be interesting.

---

## Accessibility

### Screen Reader Support

Improve accessibility:

- Proper focus announcements
- Alt text for visual elements
- Compatible with terminal screen readers

### High Contrast Mode

For users who need it:

- Dedicated high-contrast theme
- Option to disable colors entirely
- Minimum contrast ratio enforcement

---

## Wild Ideas

### Multi-Account

Switch between HN accounts (if you have multiple).

### Hacker News "Wrapped"

Year-end stats about your reading habits—most read topics, favorite authors, peak browsing times.

### Comment Threading Visualization

ASCII art tree view of comment structure, like `git log --graph`.

### "Focus Mode"

Hide scores and comment counts to reduce engagement-driven reading. Just titles and content.

### Gemini/Gopher Support

Fetch and display stories from text-only protocols for linked content.
