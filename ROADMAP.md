# Roadmap

Ideas for future features. The priority list ranks by value, effort, and whether a feature enables others. Detailed
descriptions are in the sections below.

---

## Completed

| Feature        | Notes                                                                        |
|----------------|------------------------------------------------------------------------------|
| Pagination     | Infinite scroll with automatic loading as you navigate. Background fetching. |
| Debug Mode     | Toggle with backtick (`). Shows running tasks and event log.                 |
| SQLite Storage | Local cache in `~/.local/share/hn/hn.db`. Write-through caching, async ops.  |
| Algolia API    | Single-request comment fetching via Algolia. Firebase fallback for resiliency.|

---

## Priority

| #  | Feature                  | Rationale                                                                     |
|----|--------------------------|-------------------------------------------------------------------------------|
| 1  | Read/Unread Tracking     | Transforms app from viewer to daily-driver. SQLite storage ready.             |
| 2  | Local Bookmarks          | Completes core reading workflow. SQLite storage ready.                        |
| 3  | Copy to Clipboard        | Quick win, immediate utility. Users constantly want to share links.           |
| 4  | Comment Enhancements     | Highlight OP, jump between top-level—small effort, better reading experience. |
| 5  | View History             | Natural extension of read tracking. SQLite storage ready.                     |
| 6  | Search                   | High value for finding old discussions. Algolia API already integrated.       |
| 7  | User Profiles            | View karma, submissions, comments. Useful context when reading threads.       |
| 8  | Story Filtering          | Hide low-score stories, block domains. Personalization without account.       |
| 9  | Mouse Support            | Events already captured. Click-to-select, scroll wheel. Quick win.            |
| 10 | Status Bar Improvements  | Polish: position indicator, time since refresh, unread count.                 |
| 11 | Background Refresh       | Keep feeds fresh automatically. Nice for leaving app open.                    |
| 12 | Preloading               | Prefetch next page, comments for nearby stories. Snappier feel.               |
| 13 | Code Block Formatting    | Better rendering for technical discussions. Moderate effort.                  |
| 14 | Customizable Keybindings | Power user feature. Config file for remapping keys.                           |
| 15 | Export Thread            | Save discussions as markdown. Useful for reference.                           |
| 16 | Split View               | Stories + comments side-by-side. Ambitious UI change.                         |
| 17 | Link Preview             | Fetch page title/description. Opt-in for privacy. Nice-to-have.               |
| 18 | Offline Mode             | SQLite in place makes this straightforward. Query local DB when offline.      |
| 19 | Login Support            | Cookie-based auth. Enables upvoting/commenting. Security-sensitive.           |
| 20 | Upvoting                 | Requires login. Visual indicator for upvoted items.                           |
| 21 | Commenting & Replies     | Requires login. Compose in $EDITOR. Most complex account feature.             |
| 22 | Share Integration        | Platform-specific (macOS share sheet, etc.). Limited audience.                |
| 23 | Screen Reader Support    | Accessibility: focus announcements, terminal reader compat.                   |
| 24 | High Contrast Mode       | Accessibility: dedicated theme, disable colors option.                        |
| 25 | Plugin System            | Lua/WASM extensibility. Very ambitious, likely overkill.                      |
| 26 | Focus Mode               | Hide scores/counts. Niche but interesting for mindful reading.                |
| 27 | Multi-Account            | Switch HN accounts. Very niche use case.                                      |
| 28 | Comment Threading Viz    | ASCII tree view like `git log --graph`. Fun but niche.                        |
| 29 | HN "Wrapped"             | Reading stats. Fun year-end feature, requires history first.                  |
| 30 | Gemini/Gopher Support    | Text-protocol fetching. Very niche.                                           |

---

## SQLite Storage ✓

Implemented with write-through caching—items are stored as they're fetched from the API. Uses rusqlite with embedded
migrations. Database location: `~/.local/share/hn/hn.db`. Schema includes `items` table (stories, comments, polls, jobs)
and `feeds`/`feed_items` tables for cached feed orderings. Async writes ensure the UI stays responsive.

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

### Deep Comment Loading ✓

Solved by switching to Algolia API for comment fetching. Algolia returns the entire comment tree in a single request,
eliminating the depth limit. Firebase API is used as a fallback if Algolia is unavailable.

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
