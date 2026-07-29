use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    #[arg(short, long, value_parser = validate_file_exists)]
    pub config: PathBuf,
}

/// Validate that the given file exists.
fn validate_file_exists(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if !path.exists() {
        return Err(format!("File doesn't exist: {s}"));
    }
    if !path.is_file() {
        return Err(format!("Not a file: {s}"));
    }
    Ok(path)
}
