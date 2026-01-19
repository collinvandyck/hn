# Plan: Agent-Operated TUI Testing

## Problem

Snapshot tests verify rendered output at fixed states, but can't verify:
- Interactive flows (navigation sequences, state transitions)
- Error recovery paths
- Real keyboard input handling
- Async behavior timing

We want an AI agent (Claude Code) to operate the TUI as a black box: launch it, send keystrokes, observe the screen, and make decisions.

## Approach Options

### Option A: rexpect (Recommended)

**What it is:** Rust port of pexpect. Spawns processes in a PTY, sends input, pattern-matches output.

**Pros:**
- Pure Rust, integrates naturally with the test suite
- Mature (51 dependents on crates.io)
- Zero changes to the TUI itself
- Can be used from integration tests

**Cons:**
- Raw ANSI output requires parsing/emulation to "see" the screen
- Pattern matching on escape sequences is fragile

**Usage pattern:**
```rust
use rexpect::spawn;

let mut p = spawn("./target/debug/hn", Some(5000))?;
p.exp_string("Top")?;              // Wait for feed tab
p.send("j")?;                       // Move down
p.send("l")?;                       // Open comments
p.exp_regex(r"\d+ comments")?;      // Verify comments loaded
p.send("q")?;                       // Quit
p.exp_eof()?;
```

**Screen state:** Combine with `vt100` crate to emulate terminal and get actual screen buffer:
```rust
let mut parser = vt100::Parser::new(24, 80, 0);
parser.process(output_bytes);
let screen = parser.screen();
let text = screen.contents();  // Get rendered text
```

### Option B: ratatui-testlib

**What it is:** Integration testing library specifically for ratatui apps. Runs TUI in PTY with terminal emulation built-in.

**Pros:**
- Designed for exactly this use case
- Built-in screen state querying (`text_at`, `cursor_position`)
- Integrates with insta snapshots

**Cons:**
- Newer/less mature (v0.1.0)
- May require some TUI modifications for harness integration
- Focused on Bevy ECS use cases currently

**Usage pattern:**
```rust
let mut harness = TuiTestHarness::new(80, 24)?;
harness.spawn(Command::new("./target/debug/hn"))?;
harness.wait_for(|state| state.contents().contains("Top"))?;
harness.send_key(Key::Char('j'))?;
```

### Option C: interminai (External Tool)

**What it is:** PTY wrapper with socket-based API designed for AI agent interaction.

**Pros:**
- Explicitly designed for AI agents
- Daemon mode for long-running sessions
- Simple CLI interface

**Cons:**
- External dependency (not Rust crate)
- Socket IPC adds complexity
- GPL-2.0 license

**Usage pattern (from shell):**
```bash
# Start TUI
interminai start --socket /tmp/hn.sock -- ./target/debug/hn

# AI agent reads screen
interminai output --socket /tmp/hn.sock

# AI agent sends input
interminai input --socket /tmp/hn.sock --text "j"

# Cleanup
interminai stop --socket /tmp/hn.sock
```

### Option D: Custom Test Harness

**What it is:** Build a thin wrapper that exposes the TUI's internal state for testing.

**Pros:**
- Full control
- Can expose exactly what the agent needs

**Cons:**
- Requires TUI modifications (violates "black box" requirement)
- Maintenance burden

---

## Recommended Approach: rexpect + vt100

This keeps the TUI as a true black box while giving the agent full interaction capability.

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Test Harness                         │
│  ┌─────────────┐   ┌──────────────┐   ┌─────────────┐  │
│  │   rexpect   │──▶│   vt100      │──▶│   Screen    │  │
│  │  (PTY I/O)  │   │  (emulator)  │   │   State     │  │
│  └─────────────┘   └──────────────┘   └─────────────┘  │
│         │                                    │          │
│         │  send_line("j")                    │          │
│         ▼                                    ▼          │
│  ┌─────────────┐                     ┌─────────────┐   │
│  │    hn TUI   │                     │  Assertions │   │
│  │ (black box) │                     │  / Queries  │   │
│  └─────────────┘                     └─────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### Implementation Plan

#### Phase 1: Basic Harness

1. Add dev-dependencies:
   ```toml
   [dev-dependencies]
   rexpect = "0.6"
   vt100 = "0.16"
   ```

2. Create `tests/harness.rs`:
   ```rust
   pub struct TuiHarness {
       process: rexpect::session::PtySession,
       parser: vt100::Parser,
   }

   impl TuiHarness {
       pub fn spawn() -> Result<Self> { ... }
       pub fn send_key(&mut self, key: char) -> Result<()> { ... }
       pub fn send_keys(&mut self, keys: &str) -> Result<()> { ... }
       pub fn screen_text(&mut self) -> String { ... }
       pub fn wait_for(&mut self, pattern: &str) -> Result<()> { ... }
       pub fn quit(&mut self) -> Result<()> { ... }
   }
   ```

3. Create `tests/integration.rs`:
   ```rust
   #[test]
   fn test_navigation_flow() {
       let mut h = TuiHarness::spawn().unwrap();
       h.wait_for("Top").unwrap();

       h.send_key('j');  // Move down
       h.send_key('j');
       h.send_key('l');  // Open comments

       h.wait_for("comments").unwrap();

       h.send_key('q');  // Back
       h.send_key('q');  // Quit
   }
   ```

#### Phase 2: Agent-Friendly Interface

Create a simple protocol for Claude Code to interact:

1. **Script runner** (`tests/agent_runner.rs`):
   - Accepts a script file with commands
   - Outputs screen state as text after each command
   - Example script format:
     ```
     wait Top
     key j
     key j
     key l
     wait comments
     screenshot
     key q
     key q
     ```

2. **Bash tool integration**:
   ```bash
   # Agent runs:
   cargo run --bin agent-harness -- script.txt

   # Gets back screen states for verification
   ```

#### Phase 3: Agent Harness with Unix Socket IPC

For Claude Code to operate the TUI directly, we use a Unix domain socket for bidirectional communication.
Agent connects, sends a command, reads the response on the same connection.

##### Architecture

```
┌─────────────────┐                      ┌─────────────────────────────────┐
│   Claude Code   │                      │         Agent Harness           │
│     (Agent)     │                      │                                 │
│                 │   /tmp/hn.sock       │  ┌─────────┐    ┌───────────┐  │
│  Send command ──┼──────────────────────┼─▶│ Command │───▶│  TUI via  │  │
│                 │   (Unix socket)      │  │ Parser  │    │  rexpect  │  │
│  Recv response ◀┼──────────────────────┼──┤         │◀───│           │  │
│                 │                      │  └─────────┘    └───────────┘  │
│                 │                      │        (screen via vt100)      │
└─────────────────┘                      └─────────────────────────────────┘
```

##### Protocol

JSON-over-newline: each message is a single line of JSON, terminated by newline.

**Command types:**

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
enum Command {
    Screen,
    Keys { keys: String },
    Ctrl { char: char },
    Wait { pattern: String, #[serde(default = "default_timeout")] timeout_ms: u64 },
    Quit,
}

fn default_timeout() -> u64 { 5000 }
```

**Response types:**

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Response {
    Ok,
    OkTimed { elapsed_ms: u64 },
    Screen { rows: u16, cols: u16, content: String },
    Error { message: String },
}
```

**Example messages:**

```json
// Commands (agent sends):
{"cmd": "screen"}
{"cmd": "keys", "keys": "j"}
{"cmd": "keys", "keys": "jjjl"}
{"cmd": "ctrl", "char": "c"}
{"cmd": "wait", "pattern": "comments", "timeout_ms": 3000}
{"cmd": "quit"}

// Responses (harness sends):
{"status": "ok"}
{"status": "ok_timed", "elapsed_ms": 245}
{"status": "screen", "rows": 24, "cols": 80, "content": "..."}
{"status": "error", "message": "timeout after 5000ms"}
```

##### Agent Workflow

```bash
# 1. Start harness in background (creates socket, spawns TUI)
cargo run --bin agent-harness -- --socket /tmp/hn.sock &
sleep 1  # Wait for socket to be ready

# 2. Interact using netcat (or socat)
echo '{"cmd": "screen"}' | nc -U /tmp/hn.sock

# 3. Navigate
echo '{"cmd": "keys", "keys": "j"}' | nc -U /tmp/hn.sock
echo '{"cmd": "keys", "keys": "l"}' | nc -U /tmp/hn.sock

# 4. Wait for async content
echo '{"cmd": "wait", "pattern": "comments", "timeout_ms": 3000}' | nc -U /tmp/hn.sock

# 5. See the result
echo '{"cmd": "screen"}' | nc -U /tmp/hn.sock

# 6. Shutdown
echo '{"cmd": "quit"}' | nc -U /tmp/hn.sock
```

##### Implementation

**`src/bin/agent-harness.rs`**:

```rust
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::Instant;
use anyhow::Result;
use rexpect::session::PtySession;
use serde::{Deserialize, Serialize};
use vt100::Parser;

// ─── Protocol Types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
enum Command {
    Screen,
    Keys { keys: String },
    Ctrl { char: char },
    Wait {
        pattern: String,
        #[serde(default = "default_timeout")]
        timeout_ms: u64,
    },
    Quit,
}

fn default_timeout() -> u64 { 5000 }

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Response {
    Ok,
    OkTimed { elapsed_ms: u64 },
    Screen { rows: u16, cols: u16, content: String },
    Error { message: String },
}

// ─── Harness ────────────────────────────────────────────────────────────────

struct Harness {
    pty: PtySession,
    parser: Parser,
    socket_path: PathBuf,
    listener: UnixListener,
}

impl Harness {
    fn new(socket_path: &str, width: u16, height: u16) -> Result<Self> {
        let socket_path = PathBuf::from(socket_path);
        let _ = fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path)?;
        let pty = rexpect::spawn("./target/debug/hn", Some(30_000))?;
        let parser = Parser::new(height, width, 0);
        Ok(Self { pty, parser, socket_path, listener })
    }

    fn run(&mut self) -> Result<()> {
        loop {
            let (stream, _) = self.listener.accept()?;
            if self.handle_connection(stream)? {
                return Ok(());
            }
        }
    }

    fn handle_connection(&mut self, stream: UnixStream) -> Result<bool> {
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut writer = stream;
        let mut line = String::new();

        while reader.read_line(&mut line)? > 0 {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                line.clear();
                continue;
            }

            let (response, should_quit) = match serde_json::from_str::<Command>(trimmed) {
                Ok(cmd) => self.handle_command(cmd),
                Err(e) => (Response::Error { message: e.to_string() }, false),
            };

            let json = serde_json::to_string(&response)?;
            writeln!(writer, "{}", json)?;
            writer.flush()?;

            if should_quit {
                return Ok(true);
            }
            line.clear();
        }
        Ok(false)
    }

    fn handle_command(&mut self, cmd: Command) -> (Response, bool) {
        match cmd {
            Command::Screen => {
                if let Err(e) = self.update_screen() {
                    return (Response::Error { message: e.to_string() }, false);
                }
                let screen = self.parser.screen();
                let (rows, cols) = screen.size();
                (Response::Screen {
                    rows,
                    cols,
                    content: screen.contents(),
                }, false)
            }
            Command::Keys { keys } => {
                for c in keys.chars() {
                    if let Err(e) = self.pty.send(&c.to_string()) {
                        return (Response::Error { message: e.to_string() }, false);
                    }
                }
                let _ = self.pty.flush();
                (Response::Ok, false)
            }
            Command::Ctrl { char } => {
                if let Err(e) = self.pty.send_control(char) {
                    return (Response::Error { message: e.to_string() }, false);
                }
                (Response::Ok, false)
            }
            Command::Wait { pattern, timeout_ms } => {
                let start = Instant::now();
                loop {
                    if let Err(e) = self.update_screen() {
                        return (Response::Error { message: e.to_string() }, false);
                    }
                    if self.parser.screen().contents().contains(&pattern) {
                        return (Response::OkTimed {
                            elapsed_ms: start.elapsed().as_millis() as u64,
                        }, false);
                    }
                    if start.elapsed().as_millis() > timeout_ms as u128 {
                        return (Response::Error {
                            message: format!("timeout after {}ms", timeout_ms),
                        }, false);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
            Command::Quit => {
                let _ = self.pty.send("q");
                (Response::Ok, true)
            }
        }
    }

    fn update_screen(&mut self) -> Result<()> {
        if let Ok(output) = self.pty.try_read() {
            self.parser.process(&output);
        }
        Ok(())
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket_path);
    }
}

fn main() -> Result<()> {
    let socket_path = std::env::args().nth(2).unwrap_or("/tmp/hn.sock".into());
    let mut harness = Harness::new(&socket_path, 80, 24)?;
    harness.run()
}
```

##### Example Agent Session

Claude Code operating the TUI:

```bash
# Start harness
$ cargo run --bin agent-harness -- --socket /tmp/hn.sock &
[1] 12345

# Get initial screen
$ echo '{"cmd": "screen"}' | nc -U /tmp/hn.sock
{"status":"screen","rows":24,"cols":80,"content":" Top   New   Best..."}

# Navigate down
$ echo '{"cmd": "keys", "keys": "j"}' | nc -U /tmp/hn.sock
{"status":"ok"}

# Open comments
$ echo '{"cmd": "keys", "keys": "l"}' | nc -U /tmp/hn.sock
{"status":"ok"}

# Wait for comments to load
$ echo '{"cmd": "wait", "pattern": "comments", "timeout_ms": 3000}' | nc -U /tmp/hn.sock
{"status":"ok_timed","elapsed_ms":245}

# See comments view
$ echo '{"cmd": "screen"}' | nc -U /tmp/hn.sock
{"status":"screen","rows":24,"cols":80,"content":" ← Show HN: I built..."}

# Done
$ echo '{"cmd": "quit"}' | nc -U /tmp/hn.sock
{"status":"ok"}
```

### Files to Create

| File | Purpose |
|------|---------|
| `src/bin/agent-harness.rs` | FIFO-based harness for agent interaction |
| `tests/harness/mod.rs` | TuiHarness struct for integration tests |
| `tests/harness/screen.rs` | Screen state parsing and queries |
| `tests/integration/mod.rs` | Integration test cases |

### Dependencies to Add

```toml
[dev-dependencies]
rexpect = "0.6"
vt100 = "0.16"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

Note: `serde` is likely already a dependency. Unix sockets use `std::os::unix::net` from the standard library.

---

## Recommendation

**Start with Option A (rexpect + vt100)** because:

1. Zero TUI modifications (true black box)
2. Pure Rust integration with existing test infrastructure
3. Mature libraries with good documentation
4. Can evolve into agent-friendly CLI tool in Phase 3

The shell-based tmux approach is a good fallback for quick experiments.

## Next Steps

1. Add `rexpect` and `vt100` to dev-dependencies
2. Implement basic `TuiHarness`
3. Write one integration test proving the concept
4. Iterate on agent-friendly interface based on real usage

## Sources

- [rexpect](https://docs.rs/rexpect) - Rust pexpect port
- [ratatui-testlib](https://lib.rs/crates/ratatui-testlib) - Ratatui integration testing
- [interminai](https://github.com/mstsirkin/interminai) - AI-focused PTY wrapper
- [Testing TUI Apps](https://blog.waleedkhan.name/testing-tui-apps/) - Background on PTY testing
- [pexpect](https://pexpect.readthedocs.io/en/stable/) - Original Python library
