# Plan: Agent-Operated TUI Testing

## Worktree

All implementation work for this feature should be done in the worktree:

```
/Users/collin/code/hn/worktrees/agent-harness  [agent-harness]
```

## Problem

Snapshot tests verify rendered output at fixed states, but can't verify:
- Interactive flows (navigation sequences, state transitions)
- Error recovery paths
- Real keyboard input handling
- Async behavior timing

We want an AI agent (Claude Code) to operate the TUI as a black box: launch it, send keystrokes, observe the screen, and make decisions.

## Approach Options

### Option A: portable-pty + vt100 (Implemented)

**What it is:** `portable-pty` spawns processes in a properly-sized PTY, `vt100` emulates terminal state.

**Pros:**
- Pure Rust, integrates naturally
- Proper PTY size handling (critical for TUI rendering)
- Handles UTF-8 correctly (important for unicode characters like spinners)
- Zero changes to the TUI itself

**Cons:**
- Raw ANSI output requires parsing/emulation to "see" the screen

**Note:** We initially tried `rexpect` but it had UTF-8 handling issues with unicode braille spinner characters (⠋⠙⠹...), causing panics. `portable-pty` handles this correctly.

**Usage pattern:**
```rust
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use vt100::Parser;

let pty_system = native_pty_system();
let pair = pty_system.openpty(PtySize {
    rows: 24,
    cols: 80,
    pixel_width: 0,
    pixel_height: 0,
})?;

let mut cmd = CommandBuilder::new("./target/release/hn");
cmd.arg("--dark");  // Skip terminal detection
let _child = pair.slave.spawn_command(cmd)?;

let mut reader = pair.master.try_clone_reader()?;
let mut writer = pair.master.take_writer()?;

// Send keystroke
writer.write_all(b"j")?;

// Read and parse output
let mut parser = Parser::new(24, 80, 0);
let mut buf = [0u8; 4096];
let n = reader.read(&mut buf)?;
parser.process(&buf[..n]);
let text = parser.screen().contents();
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

## Implemented Approach: portable-pty + vt100

This keeps the TUI as a true black box while giving the agent full interaction capability.

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Agent Harness                        │
│  ┌─────────────┐   ┌──────────────┐   ┌─────────────┐  │
│  │portable-pty │──▶│   vt100      │──▶│   Screen    │  │
│  │  (PTY I/O)  │   │  (emulator)  │   │   State     │  │
│  └─────────────┘   └──────────────┘   └─────────────┘  │
│         │                                    │          │
│         │  write_all(b"j")                   │          │
│         ▼                                    ▼          │
│  ┌─────────────┐                     ┌─────────────┐   │
│  │    hn TUI   │                     │  JSON API   │   │
│  │ (black box) │                     │ over socket │   │
│  └─────────────┘                     └─────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### Implementation Plan

#### Phase 1: Basic Harness (Completed)

1. Add dependencies (regular deps, not dev-deps, since agent-harness is a binary):
   ```toml
   [dependencies.portable-pty]
   version = "0.9"

   [dependencies.vt100]
   version = "0.15"
   ```

2. Create `src/bin/agent-harness.rs` - see actual implementation below.

#### Phase 2: Unix Socket IPC (Completed)

For Claude Code to operate the TUI directly, we use a Unix domain socket for bidirectional communication.
Agent connects, sends a command, reads the response on the same connection.

##### Architecture

```
┌─────────────────┐                      ┌─────────────────────────────────┐
│   Claude Code   │                      │         Agent Harness           │
│     (Agent)     │                      │                                 │
│                 │   /tmp/hn.sock       │  ┌─────────┐    ┌───────────┐  │
│  Send command ──┼──────────────────────┼─▶│ Command │───▶│  TUI via  │  │
│                 │   (Unix socket)      │  │ Parser  │    │portable-pty│  │
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
# 1. Start harness (waits for socket and TUI to be ready)
./scripts/harness-start /tmp/hn.sock

# 2. Get current screen state
./scripts/harness-cmd /tmp/hn.sock '{"cmd":"screen"}'

# 3. Stop when done (graceful shutdown, cleans up processes)
./scripts/harness-stop /tmp/hn.sock
```

##### Implementation

See **`src/bin/agent-harness.rs`** for the actual implementation. Key details:

- Uses `portable-pty` (not rexpect) for proper PTY handling
- Spawns TUI with `--dark` flag to skip terminal detection
- Uses absolute path via `std::env::current_exe()` for binary location
- Initial 3-second delay allows TUI to load stories

**Currently implemented commands:**
- `screen` - Returns current screen state
- `quit` - Sends 'q' keystroke and exits

**Planned commands (not yet implemented):**
- `keys` - Send keystrokes
- `ctrl` - Send control characters
- `wait` - Wait for pattern to appear

##### Example Agent Session

```bash
# Start harness (blocks until ready)
$ ./scripts/harness-start /tmp/hn.sock
/tmp/hn.sock

# Get initial screen
$ ./scripts/harness-cmd /tmp/hn.sock '{"cmd":"screen"}'
{"status":"screen","rows":24,"cols":80,"content":"[0]Favs  [1]Top  [2]New..."}

# Stop (graceful shutdown, no orphaned processes)
$ ./scripts/harness-stop /tmp/hn.sock
{"status":"ok"}
stopped
```

### Files Created

| File                        | Purpose                                     |
|-----------------------------|---------------------------------------------|
| `src/bin/agent-harness.rs`  | Unix socket harness for agent interaction   |
| `scripts/harness-start`     | Start harness, wait for socket, return path |
| `scripts/harness-cmd`       | Send JSON command, print response           |
| `scripts/harness-stop`      | Graceful shutdown with cleanup              |
| `scripts/harness-cleanup`   | Kill orphaned processes, remove sockets     |

### Helper Scripts Usage

```bash
# Start harness (blocks until ready, prints socket path)
./scripts/harness-start /tmp/hn-harness.sock

# Send commands
./scripts/harness-cmd /tmp/hn-harness.sock '{"cmd":"screen"}'

# Stop gracefully
./scripts/harness-stop /tmp/hn-harness.sock

# Clean up orphaned processes from crashed sessions
./scripts/harness-cleanup
```

### Dependencies Added

```toml
[dependencies.portable-pty]
version = "0.9"

[dependencies.vt100]
version = "0.15"
```

---

## Status

**Phase 1 (Basic Harness):** Complete - screen and quit commands working

**Phase 2 (Additional Commands):** Not started - keys, ctrl, wait commands

## Next Steps

1. Implement `keys` command for sending keystrokes
2. Implement `ctrl` command for control characters
3. Implement `wait` command for pattern matching with timeout
4. Test full navigation flow (stories → comments → back)

## Sources

- [rexpect](https://docs.rs/rexpect) - Rust pexpect port
- [ratatui-testlib](https://lib.rs/crates/ratatui-testlib) - Ratatui integration testing
- [interminai](https://github.com/mstsirkin/interminai) - AI-focused PTY wrapper
- [Testing TUI Apps](https://blog.waleedkhan.name/testing-tui-apps/) - Background on PTY testing
- [pexpect](https://pexpect.readthedocs.io/en/stable/) - Original Python library
