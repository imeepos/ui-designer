//! Rudder CLI entry point (binary name: `rudder`).
//!
//! Phase 1 scaffold: argument skeleton only, so the binary builds and
//! exposes `--version`. Full command set lands in Phase 2
//! (docs/ARCHITECTURE.md §6).

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "rudder",
    version = rudder_core::VERSION,
    about = "Rudder - AI UI design studio CLI",
    long_about = None
)]
struct Cli {
    /// Placeholder flag so the parser is wired; real globals land in Phase 2.
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

fn main() -> anyhow::Result<()> {
    let _cli = Cli::parse();
    Ok(())
}
