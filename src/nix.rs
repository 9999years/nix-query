use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::process::Command;
use std::str::FromStr;

use console::{style, StyledObject};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Deserializer};
use serde_json;

use crate::proc;
use crate::proc::CommandError;

pub const FIELD_DELIMITER: &str = "    ";

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FullLicense {
    full_name: String,
    short_name: String,
    spdx_id: Option<String>,
    url: Option<String>,
    #[serde(default = "true_")]
    free: bool,
}

impl FullLicense {
    pub fn console_fmt(&self) -> ConsoleFormatFullLicense {
        ConsoleFormatFullLicense(self)
    }
}

pub struct ConsoleFormatFullLicense<'a>(&'a FullLicense);

impl<'a> ConsoleFormatFullLicense<'a> {
    fn fmt_unfree(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let license = self.0;
        let has_full_name = license.full_name != "Unfree";
        let has_short_name = license.short_name != "unfree";

        let mut parenthetical = false;

        if has_full_name {
            write!(f, "{}", license.full_name)?;
            parenthetical = true;
            write!(f, " ({}", style("unfree").bold().red())?;
        } else {
            write!(f, "{}", style("unfree").bold().red())?;
        }

        if has_short_name {
            if parenthetical {
                write!(f, "; ")?;
            } else {
                write!(f, " (")?;
            }
            write!(f, "{})", license.short_name)?;
        } else if parenthetical {
            write!(f, ")")?;
        }

        if let Some(url_str) = &license.url {
            write!(f, " {}", url(url_str))?;
        }

        Ok(())
    }

    fn fmt_free(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let license = self.0;

        if let Some(spdx_id) = &license.spdx_id {
            write!(f, "{}", spdx_id)?;
        } else {
            write!(f, "{}", license.short_name)?;

            // No URL? Play it safe and write the full name too.
            if license.url.is_none() {
                write!(f, " ({})", license.full_name)?;
            }
        }

        if let Some(url_str) = &license.url {
            write!(f, " {}", url(url_str))?;
        }

        Ok(())
    }
}

impl Display for ConsoleFormatFullLicense<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.0.free {
            self.fmt_free(f)
        } else {
            self.fmt_unfree(f)
        }
    }
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NamedLicense {
    full_name: String,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UrlLicense {
    url: String,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum License {
    Id(String),
    Full(FullLicense),
    FullVec(Vec<FullLicense>),
    Named(NamedLicense),
    Url(UrlLicense),
}

impl License {
    pub fn console_fmt(&self) -> ConsoleFormatLicense {
        ConsoleFormatLicense(self)
    }
}

fn url<C>(s: C) -> StyledObject<C> {
    style(s).underlined().cyan()
}

fn write_licenses(licenses: &[FullLicense], f: &mut Formatter<'_>) -> fmt::Result {
    if licenses.is_empty() {
        Ok(())
    } else if licenses.len() == 1 {
        write!(f, "{}", licenses.get(0).unwrap().console_fmt())
    } else {
        for license in licenses
            .iter()
            .take(licenses.len() - 1)
            .map(FullLicense::console_fmt)
        {
            write!(f, "{}\n         ", license)?;
        }
        write!(f, "{}", licenses.last().unwrap().console_fmt())?;

        Ok(())
    }
}

pub struct ConsoleFormatLicense<'a>(&'a License);

impl Display for ConsoleFormatLicense<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            License::Id(s) => write!(f, "{}", s),
            License::Named(s) => write!(f, "{}", s.full_name),
            License::Url(s) => write!(f, "{}", url(&s.url)),
            License::Full(s) => write!(f, "{}", s.console_fmt()),
            License::FullVec(s) => write_licenses(s, f),
        }
    }
}

fn true_() -> bool {
    true
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase", try_from = "String")]
pub struct NixPath {
    path: String,
    line: usize,
}

#[derive(Debug, Clone)]
pub enum NixPathParseErr {
    BadSplit,
}

impl Display for NixPathParseErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::BadSplit => write!(f, "Nix path must contain a ':'"),
        }
    }
}

impl FromStr for NixPath {
    type Err = NixPathParseErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split(':');
        let path = split.next().ok_or(NixPathParseErr::BadSplit)?.to_string();
        let line = split
            .next()
            .ok_or(NixPathParseErr::BadSplit)?
            .parse()
            .map_err(|_| NixPathParseErr::BadSplit)?;
        Ok(NixPath { path, line })
    }
}

impl TryFrom<String> for NixPath {
    type Error = NixPathParseErr;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Key {
    longkeyid: String,
    fingerprint: String,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MaintainerInfo {
    name: Option<String>,
    email: String,
    github: Option<String>,
    github_id: Option<usize>,
    #[serde(default = "Vec::new")]
    keys: Vec<Key>,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum Maintainer {
    Name(String),
    Info(MaintainerInfo),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Platforms {
    Normal(Vec<String>),
    /// I assume this is a Mistake.
    Weird(Vec<Vec<String>>),
}

impl From<Platforms> for Vec<String> {
    fn from(p: Platforms) -> Self {
        match p {
            Platforms::Normal(v) => v,
            Platforms::Weird(vs) => vs.iter().flatten().cloned().collect(),
        }
    }
}

fn deserialize_platforms<'de, D>(d: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Platforms::deserialize(d).map(Into::into)
}

#[derive(Deserialize, Debug, PartialEq, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct NixMeta {
    #[serde(default = "true_")]
    available: bool,
    broken: bool,
    description: Option<String>,
    long_description: Option<String>,
    homepage: Option<String>, // url
    license: Option<License>,
    name: Option<String>,
    outputs_to_install: Vec<String>,
    #[serde(deserialize_with = "deserialize_platforms")]
    platforms: Vec<String>,
    position: Option<NixPath>,
    priority: Option<isize>,
    maintainers: Vec<Maintainer>,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NixInfo {
    name: String,    // gzip-1.10
    pname: String,   // gzip
    version: String, // 1.10
    system: String,  // x86_64-linux
    meta: NixMeta,
    attr: Option<String>, // nixos.gzip
}

impl NixInfo {
    pub fn console_fmt(&self) -> ConsoleFormatInfo {
        ConsoleFormatInfo(self)
    }
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase", transparent)]
pub struct AllNixInfo {
    pub attrs: HashMap<String, NixInfo>,
}

pub struct ConsoleFormatInfo<'a>(&'a NixInfo);

impl Display for ConsoleFormatInfo<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        macro_rules! write_val {
            ($f:expr, $label:expr, $val:expr) => {
                writeln!($f, "{} {}", style(format!("{}:", $label)).bold(), $val)
            };
        }

        macro_rules! write_val_opt {
            ($f:expr, $label:expr, $val:expr) => {
                if let Some(v) = $val {
                    write_val!($f, $label, v)
                } else {
                    Ok(())
                }
            };
        }

        let info = self.0;
        write_val_opt!(f, "attr", &info.attr.as_ref().map(|a| style(a).bold().green()))?;
        write_val!(f, "name", style(&info.name).bold().green())?;

        let meta = &info.meta;
        if meta.broken {
            write_val!(f, "broken", style("true").bold().red())?;
        }
        if !meta.available {
            write_val!(f, "available", style("false").bold().red())?;
        }

        write_val_opt!(f, "priority", &meta.priority)?;

        if let Some(homepage) = &meta.homepage {
            write_val!(f, "homepage", style(homepage).underlined().cyan())?;
        }

        write_val_opt!(f, "description", &meta.description)?;

        // long_description is multiline so we indent it
        if let Some(long_desc) = &meta.long_description {
            let mut lines = long_desc.lines();
            let first_line_opt = lines.next();
            if let Some(first_line) = first_line_opt {
                write_val!(f, "long desc.", first_line)?;
                for line in lines {
                    writeln!(f, "            {}", line)?;
                }
            }
        }

        write_val_opt!(
            f,
            "license",
            &meta.license.as_ref().map(License::console_fmt)
        )?;

        write_val_opt!(
            f,
            "defined in",
            &meta.position.as_ref().map(|pos| format!(
                "{} line {}",
                style(&pos.path).underlined(),
                pos.line,
            ))
        )?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum NixQueryError {
    Command(CommandError),
    /// Output was well-formed but empty. (This should not appear.)
    Empty,
}

impl From<CommandError> for NixQueryError {
    fn from(e: CommandError) -> Self {
        Self::Command(e)
    }
}

pub fn nix_query(attr: &str) -> Result<NixInfo, NixQueryError> {
    serde_json::from_str::<AllNixInfo>(&proc::run_cmd_stdout(Command::new("nix-env").args(&[
        "--query",
        "--available",
        "--json",
        "--attr",
        attr,
    ]))?)
    .map_err(CommandError::De)?
    .attrs
    .iter()
    .next()
    .ok_or(NixQueryError::Empty)
    .map(|(attr, info)| NixInfo {
        attr: Some(attr.clone()),
        ..info.clone()
    })
}

/// nix-env gives very long lines that are nicely, yet inconveniently, aligned:
/// ```plain
/// nixos._0x0                                                                0x0-2018-06-24                                                                      A client for 0x0.st
/// ```
/// That's nice. rewrite_attr_line replaces long stretches of whitespace with
/// FIELD_DELIMITER.
fn rewrite_attr_line<'a>(line: &'a str) -> Cow<'a, str> {
    lazy_static! {
        static ref RE: Regex = Regex::new(" {2,}").unwrap();
    }
    RE.replace_all(line, FIELD_DELIMITER)
}

fn rewrite_attr_lines(stdout: String) -> String {
    stdout
        .lines()
        // Attribute names starting with _ are usually meant to be "private"
        .filter(|attr| !attr.contains("._"))
        // Reformat each line
        .fold(String::with_capacity(stdout.len()), |mut acc, line| {
            acc.push_str(&rewrite_attr_line(line).trim_end());
            acc.push_str("\n");
            acc
        })
}

pub fn nix_query_all() -> Result<String, CommandError> {
    let mut args = vec!["--query", "--available", "--attr-path", "--description"];

    let mut output =
        proc::run_cmd_stdout(Command::new("nix-env").args(&args)).map(rewrite_attr_lines)?;

    // A few sub-packages don't show up by default. Is there a better way to
    // include them...?
    // TODO: Select 'nixpkgs' or 'nixos' automatically, somehow.
    let extra_attrs = &["nixpkgs.nodePackages", "nixpkgs.haskellPackages"];

    args.push("--attr");
    // We'll fill this last value in with the individual attr in the loop.
    args.push("");

    for base_attr in extra_attrs {
        args.pop();
        args.push(base_attr);
        output.push_str(
            &proc::run_cmd_stdout(Command::new("nix-env").args(&args)).map(rewrite_attr_lines)?,
        );
    }

    Ok(output)
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_deserialize_tern() {
        let tern = include_str!("../test_data/tern.json");
        assert_eq!(
            &NixInfo {
                name: "node_tern-0.24.2".to_string(),
                pname: "node_tern".to_string(),
                version: "0.24.2".to_string(),
                system: "x86_64-linux".to_string(),
                meta: NixMeta {
                    available: true,
                    description: Some("A JavaScript code analyzer for deep, cross-editor language support".to_string()),
                    homepage: Some("https://github.com/ternjs/tern#readme".to_string()),
                    license: Some(License::Id("MIT".to_string())),
                    name: Some("node_tern-0.24.2".to_string()),
                    outputs_to_install: vec!["out".to_string()],
                    position: Some(NixPath {
                        path: "/nix/store/lybqxz1h84knafw4l9mh248lfiqrw35a-nixpkgs-20.03pre210712.d8cb4ed910c/nixpkgs/pkgs/development/node-packages/node-packages-v10.nix".to_string(),
                        line: 72689,
                    }),
                    broken: false,
                    long_description: None,
                    maintainers: vec![],
                    platforms: vec![],
                    priority: None,
                },
                attr: None,
            },
            serde_json::from_str::<AllNixInfo>(tern)
                .unwrap()
                .attrs
                .get("nixpkgs.nodePackages.tern").unwrap()
        );
    }

    #[test]
    fn test_deserialize_ok() {
        let check = |s: &str, label: &str| {
            serde_json::from_str::<AllNixInfo>(s)
                .expect(&format!("Can deserialize test data for {}.", label))
                .attrs
                .values()
                .next()
                .expect(&format!(
                    "String -> NixInfo map for {} has at least one value.",
                    label
                ))
                .clone()
        };

        let _ = check(include_str!("../test_data/gcc.json"), "test_data/gcc.json");
        let _ = check(include_str!("../test_data/gzip.json"), "test_data/gcc.json");
        let _ = check(
            include_str!("../test_data/spotify.json"),
            "test_data/spotify.json",
        );
        let _ = check(
            include_str!("../test_data/tern.json"),
            "test_data/tern.json",
        );
        let _ = check(
            include_str!("../test_data/acpitool.json"),
            "test_data/acpitool.json",
        );
    }

    #[test]
    fn test_rewrite_attr_lines() {
        assert_eq!(
            format!(
                "{}\n{}\n",
                "nixpkgs.all-cabal-hashes    10e6ea0c54a4aa41de51d1d7e2314115bb2e172a.tar.gz",
                "unstable.all-cabal-hashes    10e6ea0c54a4aa41de51d1d7e2314115bb2e172a.tar.gz",
            ),
            rewrite_attr_lines(include_str!("../test_data/attrs_unfiltered.txt").to_string())
        );
    }
}
