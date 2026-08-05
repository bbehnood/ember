//! CLI entry point: `ember <file>` reads the given source file and runs it.

use std::{fs, process};

use anyhow::{Context, Result};
use ember::run;

fn main() -> Result<()> {
    // Take the file path from the first CLI argument. `args().nth(1)`
    // skips `args().nth(0)`, which is the executable path itself.
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("Usage: ember <file>");
        process::exit(1);
    };

    let source =
        fs::read(&path).with_context(|| format!("could not read '{path}'"))?;

    // `run`'s `ember::Error` implements `std::error::Error` (via
    // `thiserror`), so `?` converts it into `anyhow::Error` automatically;
    // `anyhow` then takes care of printing it nicely if `main` returns
    // `Err`.
    run(&source)?;

    Ok(())
}
