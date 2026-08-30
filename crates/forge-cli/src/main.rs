//! forge-cli: the CLI is a thin client over the Core crates (ADR 0001 —
//! this is the ONLY crate allowed to depend on clap). It formats output
//! and wires plugins in; it must never contain domain/state-machine logic
//! itself (Section 24-25).

mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "forge", version, about = "Change Integrity Engine")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize .forge/ in the current directory
    Init,
    /// Scan configured plugins and record discovered resources
    Scan,
    /// Show current state
    State {
        #[arg(long)]
        json: bool,
    },
    /// List recorded changes
    Changes {
        #[arg(long)]
        json: bool,
    },
    /// Show diff between DESIRED and OBSERVED (not yet implemented)
    Diff,
    /// Inspect a single resource
    Inspect { resource: String },
    /// Detect drift since last scan and create a plan (filesystem only in v0.1)
    Plan {
        /// Explicit operator approval, required whenever policy or Section
        /// 30 (irreversible + CRITICAL) demands it — there is no
        /// interactive approval prompt in this v0.1 CLI yet.
        #[arg(long)]
        yes: bool,
    },
    /// Apply the latest approved plan: revalidate, execute, verify, commit or compensate
    Apply,
    /// Verify a resource against expected state (not yet implemented)
    Verify,
    /// Show execution/evidence history
    History {
        #[arg(long)]
        json: bool,
    },
    /// Show evidence for a change
    Evidence { change: String },
    /// Compensate a failed execution (not yet implemented)
    Compensate,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init => commands::init::run(),
        Commands::Scan => commands::scan::run(),
        Commands::State { json } => commands::state::run(json),
        Commands::Changes { json } => commands::changes::run(json),
        Commands::Diff => commands::stub::run("diff", "requires DESIRED-vs-OBSERVED comparison (Section 7) — not yet implemented"),
        Commands::Inspect { resource } => commands::inspect::run(&resource),
        Commands::Plan { yes } => commands::plan::run(yes),
        Commands::Apply => commands::apply::run(),
        Commands::Verify => commands::stub::run("verify", "requires forge-verify wiring (Phase 8) — not yet implemented"),
        Commands::History { json } => commands::history::run(json),
        Commands::Evidence { change } => commands::evidence::run(&change),
        Commands::Compensate => commands::stub::run("compensate", "requires forge-compensate + plugin wiring (Phase 9) — not yet implemented"),
    }
}
