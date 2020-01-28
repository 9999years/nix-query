use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::string::FromUtf8Error;
use std::sync::Arc;
use std::fs::File;

use dirs;
use lazy_static::lazy_static;

/// This uniquely identifies this program (nix-query) so that our cache files
/// don't conflict with anything else.
const UUID: &str = "bfe01d7a-c700-4529-acf1-88065df2cd25";

lazy_static! {
    static ref CACHE_PATH: Option<PathBuf> = {
        Some(
            [
                dirs::cache_dir()?,
                format!("nix-query-{}.cache", UUID).into(),
            ]
            .iter()
            .collect(),
        )
    };
}

fn cache_exists() -> bool {
    CACHE_PATH.as_deref().map(Path::is_file).unwrap_or(false)
}

fn write_cache(nix_attrs: String) -> Option<()> {
    File::create(CACHE_PATH?)?.write_all(nix_attrs)?
}

enum CommandError {
    Io(Arc<io::Error>),
    Stderr(String),
    Encoding(FromUtf8Error),
    ExitStatus(ExitStatus),
}

fn run_cmd(c: &mut Command) -> Result<String, CommandError> {
    let output = c.output().map_err(Arc::new).map_err(CommandError::Io)?;

    if !output.status.success() {
        return Err(CommandError::ExitStatus(output.status));
    }

    if !output.stderr.is_empty() {
        return Err(CommandError::Stderr(
            String::from_utf8(output.stderr).map_err(CommandError::Encoding)?,
        ));
    }

    Ok(String::from_utf8(output.stdout).map_err(CommandError::Encoding)?)
}

fn nix_query_all() -> Result<String, CommandError> {
    run_cmd(Command::new("nix-env")
        .args(&["--query", "--available", "--no-name", "--attr-path"]))
}

fn main() {
    println!("Hello, world!");
}
