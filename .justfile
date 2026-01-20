alias i := install

default: run

run:
    cargo run --bin hn

rr:
    cargo run --release --bin hn

run-theme theme:
    cargo run -- --theme {{ theme }}

test:
    cargo test

snap:
    INSTA_UPDATE=1 cargo test

snap-review:
    cargo insta review

check:
    cargo check --all --tests

lint:
    cargo clippy --all -- -D warnings
    cargo clippy --all --tests -- -D warnings

fmt:
    cargo +nightly fmt

fmt-check:
    cargo +nightly fmt -- --check

build:
    cargo build --release

install:
    cargo install --path . --force

clean:
    cargo clean

ci: fmt-check lint test

# Find duplicate code blocks
duplicates:
    mise exec -- npx jscpd --reporters consoleFull --min-lines 15 src/

# Code metrics (method counts, function lengths, etc.)
metrics:
    cargo run --manifest-path tools/metrics/Cargo.toml --quiet -- src

themes:
    cargo run -- theme list

theme-show name:
    cargo run -- theme show {{ name }}

cl *args:
    lima claude --allow-dangerously-skip-permissions --dangerously-skip-permissions {{ args }}
