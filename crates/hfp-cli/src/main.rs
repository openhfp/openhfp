//! `hfp` — command-line interface for the HFP (HTML Form Package) format.
//!
//! A thin shell over [`hfp_core`]. Status: pre-alpha scaffold — the commands are
//! declared and wired to the core engine, but the engine itself is not implemented
//! yet (see the repository roadmap).

use std::process::ExitCode;

/// Declared subcommands and their one-line help, in display order.
const COMMANDS: &[(&str, &str)] = &[
    ("validate", "check whether a file is a valid HFP document"),
    ("extract", "print the embedded JSON data (machine-readable)"),
    (
        "verify",
        "verify author and data signatures (--json, --no-revocation-check)",
    ),
    (
        "canonicalize",
        "print the canonical bytes signatures are computed over (--explain)",
    ),
    ("sign", "sign a form or its data"),
    ("audit", "static heuristic scan of the form's JavaScript"),
    ("info", "print metadata, author and signature status"),
];

fn print_usage() {
    println!("hfp — HTML Form Package CLI (pre-alpha)");
    println!();
    println!("Usage: hfp <command> [options] <file>");
    println!();
    println!("Commands:");
    for (name, help) in COMMANDS {
        println!("  {name:<13} {help}");
    }
}

/// Demonstrates the thin-shell-over-core wiring: read the file and delegate to
/// [`hfp_core::verify`]. Returns a non-zero code while the engine is unimplemented.
fn run_verify(path: Option<String>) -> ExitCode {
    let Some(path) = path else {
        eprintln!("verify: missing file argument");
        return ExitCode::from(2);
    };
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    match hfp_core::verify(&bytes, &hfp_core::TrustConfig::default()) {
        Ok(report) if report.author_signature_valid && report.data_signature_valid => {
            ExitCode::SUCCESS
        }
        Ok(_) => ExitCode::from(1),
        Err(e) => {
            eprintln!("verify: {e}");
            ExitCode::from(2)
        }
    }
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None | Some("-h") | Some("--help") | Some("help") => {
            print_usage();
            ExitCode::SUCCESS
        }
        Some("-V") | Some("--version") => {
            println!("hfp {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("verify") => run_verify(args.next()),
        Some(cmd) if COMMANDS.iter().any(|(name, _)| *name == cmd) => {
            eprintln!("`{cmd}` is not implemented yet (pre-alpha scaffold).");
            ExitCode::from(2)
        }
        Some(cmd) => {
            eprintln!("unknown command: {cmd}");
            eprintln!();
            print_usage();
            ExitCode::from(2)
        }
    }
}
