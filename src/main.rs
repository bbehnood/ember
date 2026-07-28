use std::{fs, process};

use anyhow::{Context, Result};
use ember::run;

fn main() -> Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("Usage: ember <file>");
        process::exit(1);
    };

    let source =
        fs::read(&path).with_context(|| format!("could not read '{path}'"))?;

    if let Some(value) = run(&source)? {
        println!("{value}");
    }

    Ok(())
}
