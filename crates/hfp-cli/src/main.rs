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
        "print the canonical bytes signatures are computed over (--author, --sha)",
    ),
    (
        "data-payload",
        "print the bytes the data signature is computed over",
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

/// Collect the values that follow each occurrence of `flag` in `args`.
fn flag_values(args: &[String], flag: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == flag {
            if let Some(v) = it.next() {
                out.push(v.clone());
            }
        }
    }
    out
}

/// `verify <file> [--anchor DER]* [--crl DER]* [--allow-thumbprint HEX]*
///  [--require-allowed] [--no-revocation-check]`.
///
/// Reads the file, builds a [`hfp_core::TrustConfig`] from the flags, and prints the
/// report as `key=value` lines on stdout (notes to stderr). Exit 0 when both signatures
/// are valid AND the document is trusted; 1 otherwise; 2 on I/O or structural error.
fn run_verify(args: &[String]) -> ExitCode {
    let Some(path) = args.iter().find(|a| !a.starts_with("--")) else {
        eprintln!("verify: missing file argument");
        return ExitCode::from(2);
    };
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };

    let read_all = |flag: &str| -> std::io::Result<Vec<Vec<u8>>> {
        flag_values(args, flag).iter().map(std::fs::read).collect()
    };
    let trust_anchors = match read_all("--anchor") {
        Ok(v) => v,
        Err(e) => {
            eprintln!("verify: cannot read anchor: {e}");
            return ExitCode::from(2);
        }
    };
    let crls = match read_all("--crl") {
        Ok(v) => v,
        Err(e) => {
            eprintln!("verify: cannot read crl: {e}");
            return ExitCode::from(2);
        }
    };
    let trust = hfp_core::TrustConfig {
        trust_anchors,
        allowed_ca_thumbprints: flag_values(args, "--allow-thumbprint"),
        require_from_allowed_ca: args.iter().any(|a| a == "--require-allowed"),
        crls,
        no_revocation_check: args.iter().any(|a| a == "--no-revocation-check"),
    };

    match hfp_core::verify(&bytes, &trust) {
        Ok(report) => {
            println!("author_signature_valid={}", report.author_signature_valid);
            println!("data_signature_valid={}", report.data_signature_valid);
            println!("is_trusted={}", report.is_trusted);
            for note in &report.notes {
                eprintln!("  - {note}");
            }
            if report.author_signature_valid && report.data_signature_valid && report.is_trusted {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(e) => {
            eprintln!("verify: {e}");
            ExitCode::from(2)
        }
    }
}

/// Print the exact bytes the data signature is computed over (for fixture signing).
fn run_data_payload(args: &[String]) -> ExitCode {
    let Some(path) = args.iter().find(|a| !a.starts_with("--")) else {
        eprintln!("data-payload: missing file argument");
        return ExitCode::from(2);
    };
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    match hfp_core::data_signing_payload(&bytes) {
        Ok(payload) => {
            use std::io::Write;
            std::io::stdout().write_all(&payload).ok();
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("data-payload: {e}");
            ExitCode::from(2)
        }
    }
}

/// Print the canonical bytes (or, with `--sha`, their SHA-256) for a `.hfp` file.
///
/// The same code path runs natively and as a `wasm32-wasip1` module — this command is
/// the harness Spike A uses to prove the two builds produce byte-identical output.
fn run_canonicalize(args: &[String]) -> ExitCode {
    let sha_only = args.iter().any(|a| a == "--sha");
    let author = args.iter().any(|a| a == "--author");
    let Some(path) = args.iter().find(|a| !a.starts_with("--")) else {
        eprintln!("canonicalize: missing file argument");
        return ExitCode::from(2);
    };
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    let canon = if author {
        hfp_core::canonical_author_bytes(&bytes)
    } else {
        hfp_core::canonicalize(&bytes)
    };
    match canon {
        Ok(canon) => {
            if sha_only {
                let digest = hfp_core::sha256_hex(&canon);
                println!("{digest}");
            } else {
                use std::io::Write;
                std::io::stdout().write_all(&canon).ok();
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("canonicalize: {e}");
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
        Some("verify") => run_verify(&args.collect::<Vec<_>>()),
        Some("canonicalize") => run_canonicalize(&args.collect::<Vec<_>>()),
        Some("data-payload") => run_data_payload(&args.collect::<Vec<_>>()),
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
