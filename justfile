# form3 — dependency-free terminal toolkit

set fallback := true

# Run the full test suite (default)
test:
    cargo test
    cargo test --features agent
    cargo test --features cjk

# Run clippy with warnings as errors
lint:
    cargo clippy -- -D warnings
    cargo clippy --features agent -- -D warnings
    cargo clippy --features cjk -- -D warnings

# Format all Rust source
fmt:
    cargo fmt

# Build documentation
doc:
    cargo doc --no-deps --features agent,cjk

# Run the interactive TUI composer
tui:
    cargo run --example tui --features agent

# Run the agent binary for model workflows
agent:
    cargo run --bin 3form-agent --features agent

# Run all deliverable checks
deliver:
    deliver --spec deliver.toml --strict

# Clean build artifacts
clean:
    cargo clean
