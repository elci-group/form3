//! `3form-agent` — JSON-RPC stdin/stdout wrapper for the model workflow.
//!
//! Reads one newline-delimited JSON [`Request`] per line and emits a JSON
//! [`Response`] per line. No adapters, no prompts: the schema is the contract.
//!
//! # Usage
//!
//! ```bash
//! cargo run --bin 3form-agent --features agent
//! echo '{"op":"style","text":"hello","fg":"red","attrs":["bold"]}' | cargo run --bin 3form-agent --features agent
//! ```

#[cfg(feature = "agent")]
fn main() {
    use form3::agent::{handle_json_request, Agent, Request, Response};
    use std::io::{self, BufRead, Write};

    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();

    for line in stdin.lock().lines() {
        match line {
            Ok(input) => {
                if input.trim().is_empty() {
                    continue;
                }
                let mut output = String::new();
                match handle_json_request(&input, &mut output) {
                    Ok(()) => {
                        let _ = stdout.write_all(output.as_bytes());
                        let _ = stdout.flush();
                    }
                    Err(error) => {
                        let response = Response {
                            version: Agent::new().compose(Request::Reset).version,
                            output: String::new(),
                            error: Some(error),
                        };
                        let _ = serde_json::to_writer(&mut stderr, &response);
                        let _ = stderr.write_all(b"\n");
                    }
                }
            }
            Err(error) => {
                let response = Response {
                    version: "1.0.0".to_string(),
                    output: String::new(),
                    error: Some(error.to_string()),
                };
                let _ = serde_json::to_writer(&mut stderr, &response);
                let _ = stderr.write_all(b"\n");
            }
        }
    }
}

#[cfg(not(feature = "agent"))]
fn main() {
    eprintln!("3form-agent requires the `agent` feature.");
    eprintln!("Run with: cargo run --bin 3form-agent --features agent");
    std::process::exit(1);
}
