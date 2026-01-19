use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use serde::{Deserialize, Serialize};
use vt100::Parser;

// ─── Protocol Types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
enum Command {
    Screen,
    Quit,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Response {
    Ok,
    Screen {
        rows: u16,
        cols: u16,
        content: String,
    },
    Error {
        message: String,
    },
}

// ─── Harness ────────────────────────────────────────────────────────────────

struct Harness {
    reader: Box<dyn Read + Send>,
    writer: Box<dyn Write + Send>,
    parser: Parser,
    socket_path: PathBuf,
    listener: UnixListener,
}

impl Harness {
    fn new(socket_path: &str, width: u16, height: u16) -> Result<Self> {
        let socket_path = PathBuf::from(socket_path);
        let _ = fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path)?;

        // Create PTY with specified size
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: height,
            cols: width,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        // Spawn the TUI with --dark to skip terminal detection queries
        // Use absolute path to ensure it works from any CWD
        let hn_path = std::env::current_exe()?
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Cannot find parent dir"))?
            .join("hn");
        let mut cmd = CommandBuilder::new(&hn_path);
        cmd.arg("--dark");
        let _child = pair.slave.spawn_command(cmd)?;

        // Get reader/writer for the master side
        let reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;

        let parser = Parser::new(height, width, 0);
        Ok(Self {
            reader,
            writer,
            parser,
            socket_path,
            listener,
        })
    }

    fn run(&mut self) -> Result<()> {
        // Give the TUI time to start and load data
        std::thread::sleep(Duration::from_millis(3000));
        self.drain_pty()?;

        loop {
            let (stream, _) = self.listener.accept()?;
            if self.handle_connection(stream)? {
                return Ok(());
            }
        }
    }

    fn drain_pty(&mut self) -> Result<()> {
        let mut buf = [0u8; 4096];
        loop {
            std::thread::sleep(Duration::from_millis(50));
            match self.reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    self.parser.process(&buf[..n]);
                    if n < buf.len() {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
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
                Err(e) => (
                    Response::Error {
                        message: e.to_string(),
                    },
                    false,
                ),
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
                // Small delay to let any pending output arrive
                std::thread::sleep(Duration::from_millis(100));
                if let Err(e) = self.drain_pty() {
                    return (
                        Response::Error {
                            message: e.to_string(),
                        },
                        false,
                    );
                }
                let screen = self.parser.screen();
                let (rows, cols) = screen.size();
                let content = screen.contents();
                (
                    Response::Screen {
                        rows,
                        cols,
                        content,
                    },
                    false,
                )
            }
            Command::Quit => {
                let _ = self.writer.write_all(b"q");
                (Response::Ok, true)
            }
        }
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket_path);
    }
}

fn main() -> Result<()> {
    let socket_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/hn.sock".into());

    eprintln!("agent-harness: socket={}", socket_path);

    let mut harness = Harness::new(&socket_path, 80, 24)?;
    harness.run()
}
