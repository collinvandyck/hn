default: run

run:
    cargo run

rr:
    cargo run --release

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
    cargo clippy --all --tests -- -D warnings -D dead_code

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

themes:
    cargo run -- theme list

theme-show name:
    cargo run -- theme show {{ name }}

cl *args:
    lima claude --allow-dangerously-skip-permissions --dangerously-skip-permissions {{ args }}
