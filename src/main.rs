use std::io;
use std::io::Write;

use console::{style, Term};
use skim::{Skim, SkimOptionsBuilder};
use structopt::StructOpt;

use nix_query::{cache, cache::CacheIoError, nix, proc::CommandError};

#[derive(Debug)]
enum MainErr {
    Cache(CacheIoError),
    Command(CommandError),
    NixQuery(nix::NixQueryError),
    Io(io::Error),
}

impl From<io::Error> for MainErr {
    fn from(e: io::Error) -> Self {
        MainErr::Io(e)
    }
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
    about = "A tool for interactively and quickly selecting Nix packages by attribute.",
    version = "0.1.0"
)]
struct Opt {
    /// Clears and recalculates the cache.
    #[structopt(long)]
    clear_cache: bool,

    /// Prints the information for a given Nix attribute.
    #[structopt(long)]
    info: Option<String>,

    /// Print all attributes in the cache.
    #[structopt(long)]
    print_cache: bool,
}

fn main() -> Result<(), MainErr> {
    let opt = Opt::from_args();

    let mut term = Term::stdout();
    let mut eterm = Term::stderr();

    if opt.clear_cache {
        term.write_line("Clearing the Nix package name cache.")?;
        cache::clear_cache()?;
        return Ok(());
    }

    if let Some(attr) = opt.info {
        let was_using_colors = console::colors_enabled();
        console::set_colors_enabled(true);

        // write!(
        //     term,
        //     "{}",
        //     style(format!("(Querying Nix for information about {})", attr)).dim()
        // )?;
        let info = nix::nix_query(&attr)?;

        // term.clear_line()?;
        write!(term, "{}", info.console_fmt())?;

        console::set_colors_enabled(was_using_colors);
        return Ok(());
    }

    if !cache::cache_exists() {
        // Let the user know we need to populate the cache.
        writeln!(
            eterm,
            "{}",
            style("Populating the Nix package name cache (this may take a minute or two)...")
                .bold()
                .green(),
        )?;
    }

    let all_attrs = cache::ensure_cache()?;

    if opt.print_cache {
        term.write_str(&all_attrs)?;
        return Ok(());
    }

    for attr in skim_attrs()? {
        writeln!(term, "{}", first_field(&attr).unwrap_or(&attr))?;
    }

    Ok(())
}

fn first_field(s: &str) -> Option<&str> {
    s.split(' ').next()
}

fn skim_attrs() -> Result<Vec<String>, MainErr> {
    use std::env;
    use std::io::Cursor;

    let preview_cmd = format!(
        "{exe} --info {{1}}",
        exe = env::current_exe()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "nix-query".to_string()),
    );

    let options = SkimOptionsBuilder::default()
        .height(Some("100%"))
        .multi(true)
        .preview(Some(&preview_cmd))
        .preview_window(Some("down:50%"))
        .tiebreak(Some("score,end".to_string()))
        .no_hscroll(true)
        .delimiter(Some(nix::FIELD_DELIMITER))
        .nth(None) // fields to search
        .with_nth(Some("1")) // fields to show
        .build()
        .unwrap();

    let input = cache::ensure_cache()?;

    Ok(Skim::run_with(&options, Some(Box::new(Cursor::new(input))))
        .map(|out| out.selected_items)
        .map(|items| {
            items
                .iter()
                .map(|i| i.get_text())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_else(Vec::new))
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
        .unwrap_or_else(|_| panic!("Can query Nix for information about attribute {}", attr));

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
                i.attrs.values().next().unwrap_or_else(|| {
                    panic!("String -> NixInfo map for {} has at least one value.", attr)
                });
            }
        }
    }
}
