//! Workspace housekeeping. Run with `cargo run -p xtask -- <command>`.
//!
//!   check-layers    — every ember-* crate's declared deps match
//!                     `01-architecture.md §3.2`.
//!   check-macroquad — macroquad/miniquad imports appear only in the
//!                     two backend crates or in the allowlist.

mod check_layers;
mod check_macroquad;

fn main() {
    let cmd = std::env::args().nth(1).unwrap_or_else(|| "help".into());
    let result = match cmd.as_str() {
        "check-layers" => check_layers::run(),
        "check-macroquad" => check_macroquad::run(),
        "help" | "--help" | "-h" => {
            eprintln!("Usage: cargo run -p xtask -- <check-layers|check-macroquad>");
            Ok(())
        }
        other => Err(format!("unknown command: {other}")),
    };
    if let Err(e) = result {
        eprintln!("xtask: {e}");
        std::process::exit(1);
    }
}
