# Default: run the TUI in development mode
default: run

# Run the TUI in development mode
run:
    cargo run

# Run the TUI with a specific theme
run-theme theme:
    cargo run -- --theme {{theme}}

# Run all tests
test:
    cargo test

# Run tests and update snapshots automatically
snap:
    INSTA_UPDATE=1 cargo test

# Review pending snapshots interactively
snap-review:
    cargo insta review

# Check for compilation errors
check:
    cargo check --all --tests

# Run clippy lints
lint:
    cargo clippy --all --tests -- -D warnings

# Format code
fmt:
    cargo fmt

# Check formatting without modifying
fmt-check:
    cargo fmt -- --check

# Build release binary
build:
    cargo build --release

# Clean build artifacts
clean:
    cargo clean

# Run all CI checks (fmt, lint, test)
ci: fmt-check lint test

# List available themes
themes:
    cargo run -- theme list

# Show a specific theme's colors
theme-show name:
    cargo run -- theme show {{name}}
