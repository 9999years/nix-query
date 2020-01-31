use std::fs;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use dirs;
use lazy_static::lazy_static;

use crate::nix;
use crate::proc::CommandError;

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

pub const NIX_ATTRS_COUNT_ESTIMATE: usize = 100_000;
/// Bytes.
pub const NIX_ATTRS_FILE_SIZE_ESTIMATE: usize = 5_000_000;

pub fn cache_exists() -> bool {
    CACHE_PATH.as_deref().map(Path::is_file).unwrap_or(false)
}

#[derive(Debug)]
pub enum CacheIoError {
    NoCachePath,
    Command(CommandError),
    Io(Box<io::Error>),
}

impl From<io::Error> for CacheIoError {
    fn from(e: io::Error) -> CacheIoError {
        CacheIoError::Io(Box::new(e))
    }
}

pub fn clear_cache() -> Result<(), CacheIoError> {
    match fs::remove_file(CACHE_PATH.as_deref().ok_or(CacheIoError::NoCachePath)?) {
        Ok(()) => Ok(()),
        Err(io_err) => 
            // If we try to remove the cache file but it doesn't exist yet, that's OK.
            if let io::ErrorKind::NotFound = io_err.kind() {
                Ok(())
            } else {
                Err(io_err.into())
            },
    }
}

pub fn write_cache(nix_attrs: &[u8]) -> Result<(), CacheIoError> {
    File::create(CACHE_PATH.as_deref().ok_or(CacheIoError::NoCachePath)?)?
        .write_all(nix_attrs)
        .map_err(Into::into)
}

pub fn read_cache() -> Result<String, CacheIoError> {
    let mut cache_file = File::open(CACHE_PATH.as_deref().ok_or(CacheIoError::NoCachePath)?)?;
    let mut ret = String::with_capacity(NIX_ATTRS_FILE_SIZE_ESTIMATE);
    cache_file.read_to_string(&mut ret)?;
    Ok(ret)
}

pub fn ensure_cache() -> Result<String, CacheIoError> {
    if !cache_exists() {
        let attrs = nix::nix_query_all().map_err(CacheIoError::Command)?;
        write_cache(attrs.as_bytes())?;
        Ok(attrs)
    } else {
        read_cache()
    }
}
