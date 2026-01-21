# hn

a terminal ui for hacker news.

i wanted something that felt like reading hn but without leaving the terminal.
this started as an experiment with running claude in a lima vm. it ended up being
pretty ergonomic, so i put it into `--dangerously-skip-permissions` mode and spent
a couple of evenings steering it around. liking it so far. it's fun adding features
to something that you personally want.

![stories view](images/stories.png)

![comments view](images/comments.png)

## install

```
just install
```

the binary is called `hn`.

## usage

```
hn                     # run it
hn --theme dracula     # pick a theme
hn --dark              # force dark mode
hn --light             # force light mode
hn theme list          # see available themes
```

## features

- vim keybindings (j/k, h/l, o, y, g/G, ctrl-d/u, etc)
- six feeds: top, new, best, ask, show, jobs
- collapsible comment trees with depth coloring
- 12 built-in themes, or bring your own
- auto-detects terminal dark/light mode

## themes

hn comes with: monokai, dracula, nord, gruvbox, solarized, catppuccin, tokyo-night. both dark and light
variants where applicable.

custom themes go in `~/.config/hn/themes/` as toml files. run `hn theme show monokai` to see the format.

## architecture

follows elm-ish patterns: single app state, message-based updates, views are pure functions. ratatui does the rendering.
the code is reasonably organized if you need to poke around.

## about

this started as a weekend project and then kept going. it's spirit quest vibe coded, which you can interpret however you
like.
