default: run

run:
    cargo run

run-theme theme:
    cargo run -- --theme {{theme}}

test:
    cargo test

snap:
    INSTA_UPDATE=1 cargo test

snap-review:
    cargo insta review

check:
    cargo check --all --tests

lint:
    cargo clippy --all --tests -- -D warnings

fmt:
    cargo fmt

fmt-check:
    cargo fmt -- --check

build:
    cargo build --release

clean:
    cargo clean

ci: fmt-check lint test

themes:
    cargo run -- theme list

theme-show name:
    cargo run -- theme show {{name}}
