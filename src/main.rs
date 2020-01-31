use skim::{Skim, SkimOptionsBuilder};
use structopt::StructOpt;
use colored::*;

use nix_query::{cache, cache::CacheIoError, nix, proc::CommandError};

#[derive(Debug)]
enum MainErr {
    Cache(CacheIoError),
    Command(CommandError),
    NixQuery(nix::NixQueryError),
}

impl From<CacheIoError> for MainErr {
    fn from(e: CacheIoError) -> Self {
        MainErr::Cache(e)
    }
}

impl From<CommandError> for MainErr {
    fn from(e: CommandError) -> Self {
        MainErr::Command(e)
    }
}

impl From<nix::NixQueryError> for MainErr {
    fn from(e: nix::NixQueryError) -> Self {
        MainErr::NixQuery(e)
    }
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "nix-query",
)]
struct Opt {
    /// Clear and recalculate the cache.
    #[structopt(long)]
    clear_cache: bool,

    /// Print all attributes in the cache.
    #[structopt(long)]
    print_cache: bool,

    /// Prints the information for a given Nix attribute and then quit.
    #[structopt(long)]
    info: Option<String>,
}

fn main() -> Result<(), MainErr> {
    let opt = Opt::from_args();

    if opt.clear_cache {
        cache::clear_cache()?;
    }

    if let Some(attr) = opt.info {
        colored::control::set_override(true);
        print!("{}", nix::nix_query(&attr)?.console_fmt());
        colored::control::unset_override();
        return Ok(());
    }

    if !cache::cache_exists() {
        eprintln!(
            "{}",
            "Populating the Nix package name cache (this may take a minute or two)..."
                .bold()
                .green(),
        );
    }
    let all_attrs = cache::ensure_cache()?;

    if opt.print_cache {
        print!("{}", all_attrs);
        return Ok(());
    }

    skim_attrs()?;

    Ok(())
}

fn skim_attrs() -> Result<(), MainErr> {
    use std::env;
    use std::io::Cursor;

    let preview_cmd = format!(
        "{exe} --info {{}}",
        exe = env::current_exe()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "nix-query".to_string()),
    );

    let options = SkimOptionsBuilder::default()
        .height(Some("100%"))
        .multi(true)
        .preview(Some(&preview_cmd))
        .preview_window(Some("down:wrap:50%"))
        .tiebreak(Some("score,end".to_string()))
        .build()
        .unwrap();

    let input = cache::ensure_cache()?;

    let selected_items = Skim::run_with(&options, Some(Box::new(Cursor::new(input))))
        .map(|out| out.selected_items)
        .unwrap_or_else(Vec::new);

    for item in selected_items.iter() {
        println!("{}", item.get_output_text());
    }

    Ok(())
}

pub fn check_pkg_schemas() {
    use std::process::Command;

    use nix_query::proc;

    println!("Reading cache.");
    let mut lines: Vec<String> = cache::ensure_cache()
        .expect("Can read from cache")
        .lines()
        .by_ref()
        .map(|s| s.to_string())
        .collect();

    println!("Sorting cache.");
    lines.sort_unstable();

    println!("Checking.");
    let mut skipping = true;
    for (inx, attr) in lines.iter().enumerate() {
        if attr.starts_with("nixpkgs._")
            || attr.starts_with("nixos._")
            || attr.starts_with("unstable._")
        {
            continue;
        }

        if skipping {
            if attr.starts_with("nixpkgs.lzip") {
                skipping = false;
            } else {
                continue;
            }
        }

        let json = proc::run_cmd_stdout(Command::new("nix-env").args(&[
            "--query",
            "--available",
            "--json",
            "--attr",
            &attr,
        ]))
        .unwrap_or_else(|_| panic!(
            "Can query Nix for information about attribute {}",
            attr
        ));

        match serde_json::from_str::<nix::AllNixInfo>(&json) {
            Err(d) => {
                println!("{} | {}\n\t{}", attr, d, json);
            }
            Ok(i) => {
                println!(
                    "OK: {attr} [{inx}/{len}]",
                    attr = attr,
                    inx = inx,
                    len = lines.len(),
                );
                i.attrs.values().next().unwrap_or_else(|| panic!(
                    "String -> NixInfo map for {} has at least one value.",
                    attr
                ));
            }
        }
    }
}
