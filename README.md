# form3

Dependency-free ANSI terminal styling, tables, and animation primitives for Rust.

- **Zero runtime dependencies.** Everything is built on `std` plus in-tree vendored Unicode width tables.
- **SOTA correctness.** ANSI SGR, 256/truecolor, OSC 8 hyperlinks, Unicode-aware table layout, and standard spinner sets.
- **Two workflows.** A human TUI/IDE experience and a model-facing JSON-RPC agent API.

## Quick start

```rust
use form3::compat::Colorize;
use form3::table::{Table, Cell, TableStyle};

fn main() {
    let banner = "form3".bright_cyan().bold().to_string();
    println!("{banner}");

    let mut table = Table::new();
    table.set_style(TableStyle::Rounded);
    table.set_header(vec![Cell::new("Tool"), Cell::new("Status")]);
    table.add_row(vec![Cell::new("form3"), Cell::new("ok").green()]);
    println!("{table}");
}
```

## Features

- `cjk` — treat ambiguous-width characters as 2 columns (CJK context).
- `agent` — typed JSON-RPC surface for model workflows (pulls in `serde`/`serde_json`).

## Human workflow

### TUI composer

Run the interactive terminal UI:

```bash
cargo run --example tui
```

### IDE support

VS Code tasks and snippets are provided under `.vscode/`. Common commands are also available via `just`:

```bash
just test      # run all tests, all feature combinations
just lint      # clippy with warnings as errors
just tui       # run the TUI composer
just agent     # run the agent binary
just deliver   # run deliverable checks
```

## Model workflow

Models interact with `form3` through the `3form-agent` binary. The API call loop is the primary surface; no adapters are required.

```bash
cargo run --bin 3form-agent --features agent
```

Send one newline-delimited JSON request per line:

```json
{"op":"style","text":"hello","fg":"red","attrs":["bold"]}
{"op":"table","header":["Tool","Status"],"rows":[["form3","ok"]],"style":"rounded"}
{"op":"spinner","tick":0}
{"op":"progress","length":100,"position":42}
```

Each line returns a JSON response:

```json
{"version":"1.0.0","output":"\u001b[31m\u001b[1mhello\u001b[0m","error":null}
```

The in-process Rust API is also available:

```rust
#[cfg(feature = "agent")]
use form3::agent::{Agent, Request};

#[cfg(feature = "agent")]
fn demo() {
    let agent = Agent::new();
    let response = agent.compose(Request::Reset);
    println!("{}", response.output);
}
```

## Architecture

- `src/ansi.rs` — raw SGR/OSC sequence construction.
- `src/compat.rs` — `Colorize`-style chained styling API.
- `src/table.rs` — Unicode-aware table layout and rendering.
- `src/anim.rs` — spinners, progress bars, and render surfaces.
- `src/term.rs` — terminal capability detection (`NO_COLOR`, `COLORTERM`, etc.).
- `src/width.rs` + `src/width_tables.rs` — vendored Unicode display-width engine.
- `src/agent.rs` — typed model/agent API surface (feature-gated).

## Development tooling

This project uses the elci-group agent toolchain:

- `kaptaind` — semantic versioning and deterministic commits.
- `deliver` — deliverable validation via `deliver.toml`.
- `fract` — architectural health monitoring.
- `diverge` — multi-frame design review.
- `verify-chain` — final output audit.
- `tokaudit` — token-efficiency review.

## License

MIT OR Apache-2.0.

The Unicode width tables in `src/width_tables.rs` are vendored from `unicode-width` v0.2.2 and remain under their original MIT/Apache-2.0 license.
