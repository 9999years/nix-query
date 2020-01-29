use std::borrow::Borrow;
use std::convert::{TryFrom, TryInto};
use std::fs;
use std::fs::{File, Metadata};
use std::io;
use std::io::Cursor;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output, Stdio};
use std::str::FromStr;
use std::string::FromUtf8Error;
use std::sync::Arc;

use dirs;
use lazy_static::lazy_static;
use skim::{Skim, SkimOptionsBuilder};
use structopt::StructOpt;

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

const NIX_ATTRS_COUNT_ESTIMATE: usize = 100_000;

fn cache_exists() -> bool {
    CACHE_PATH.as_deref().map(Path::is_file).unwrap_or(false)
}

#[derive(Debug)]
enum CacheIoError {
    NoCachePath,
    Io(Box<io::Error>),
}

impl From<io::Error> for CacheIoError {
    fn from(e: io::Error) -> CacheIoError {
        CacheIoError::Io(Box::new(e))
    }
}

fn clear_cache() -> Result<(), CacheIoError> {
    fs::remove_file(CACHE_PATH.as_deref().ok_or(CacheIoError::NoCachePath)?).map_err(Into::into)
}

fn write_cache(nix_attrs: &[u8]) -> Result<(), CacheIoError> {
    File::create(CACHE_PATH.as_deref().ok_or(CacheIoError::NoCachePath)?)?
        .write_all(nix_attrs)
        .map_err(Into::into)
}

type NixAttrs = Vec<String>;

fn read_cache() -> Result<NixAttrs, CacheIoError> {
    let mut cache_file = File::open(CACHE_PATH.as_deref().ok_or(CacheIoError::NoCachePath)?)?;
    let mut ret = Vec::with_capacity(NIX_ATTRS_COUNT_ESTIMATE);
    ret.extend(
        BufReader::new(cache_file)
            .lines()
            .collect::<Result<_, io::Error>>()
            .map_err(Into::<CacheIoError>::into),
    );
    Ok(ret)
}

fn ensure_cache() -> Result<NixAttrs, CacheIoError> {
    if !cache_exists() {
        write_cache(&[])?; // TODO: fix this
    }
    read_cache()
}

#[derive(Debug)]
enum CommandError {
    Io(Box<io::Error>),
    Stderr(String),
    Encoding(FromUtf8Error),
    ExitStatus(ExitStatus),
}

impl From<io::Error> for CommandError {
    fn from(e: io::Error) -> Self {
        CommandError::Io(Box::new(e))
    }
}

fn run_cmd<F, T>(c: &mut Command, f: F) -> Result<T, CommandError>
where
    F: FnOnce(Vec<u8>) -> T,
{
    let output = c.output().map_err(Box::new).map_err(CommandError::Io)?;

    if !output.status.success() {
        return Err(CommandError::ExitStatus(output.status));
    }

    if !output.stderr.is_empty() {
        return Err(CommandError::Stderr(
            String::from_utf8(output.stderr).map_err(CommandError::Encoding)?,
        ));
    }

    Ok(f(output.stdout))
}

fn run_cmd_stdout(c: &mut Command) -> Result<String, CommandError> {
    run_cmd(c, |stdout| String::from_utf8(stdout))?.map_err(CommandError::Encoding)
}

fn run_cmd_stdout_lines_capacity(
    c: &mut Command,
    lines_hint: usize,
) -> Result<Vec<String>, CommandError> {
    let mut ret = Vec::with_capacity(lines_hint);
    ret.extend(run_cmd(c, |stdout| {
        stdout
            .lines()
            .collect::<Result<_, io::Error>>()
            .map_err(Into::<CommandError>::into)
    })?);
    Ok(ret)
}
fn run_cmd_stdout_lines(c: &mut Command) -> Result<Vec<String>, CommandError> {
    run_cmd_stdout_lines_capacity(c, 64)
}

struct License {
    fullName: String,
    shortName: String,
    spdxId: Option<String>,
    url: Option<String>,
    free: bool, // default = true
}

struct NixPath {
    path: String,
    line: usize,
}

impl FromStr for NixPath {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split(':');
        let path = split.next().ok_or(())?.to_string();
        let line = split.next().ok_or(())?.parse().map_err(|_| ())?;
        Ok(NixPath { path, line })
    }
}

struct Maintainer {
    email: String,
    github: String,
    githubId: usize,
    name: String,
}

struct NixMeta {
    available: bool,
    broken: bool, // default = false
    description: String,
    longDescription: Option<String>,
    homepage: String, // url
    license: License,
    name: String,
    outputsToInstall: Vec<String>,
    platforms: Vec<String>,
    position: NixPath,
    priority: Option<usize>,
    maintainers: Vec<Maintainer>,
}

struct NixInfo {
    name: String,    // gzip-1.10
    pname: String,   // gzip
    version: String, // 1.10
    system: String,  // x86_64-linux
    meta: NixMeta,
}

fn nix_query(attr: &str) -> Result<String, CommandError> {
    run_cmd_stdout(Command::new("nix-env").args(&[
        "--query",
        "--available",
        "--json",
        "--attr",
        attr,
    ]))
}

fn nix_query_all() -> Result<NixAttrs, CommandError> {
    run_cmd_stdout_lines_capacity(
        Command::new("nix-env").args(&["--query", "--available", "--no-name", "--attr-path"]),
        NIX_ATTRS_COUNT_ESTIMATE,
    )
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "nix-query",
    about = "A tool for interactively and quickly selecting Nix packages by attribute."
)]
struct Opt {
    /// Clears and recalculates the cache.
    #[structopt(long)]
    clear_cache: bool,
}

#[derive(Debug)]
enum MainErr {
    Cache(CacheIoError),
}

fn main() -> Result<(), MainErr> {
    let opt = Opt::from_args();
    if opt.clear_cache {
        clear_cache().map_err(MainErr::Cache)?;
    }
    Ok(())
}
