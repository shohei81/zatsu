mod outline;

use std::fs;
use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser, error::ErrorKind};
use outline::write_outline;
use zatsu::lang_for_ext;
use zatsu::repository::{
    RepositoryOutlineOptions, truncation_message, write_repository_outline_with_options,
};

#[derive(Parser)]
#[command(
    version,
    about = "See a repository or source file at a glance",
    after_help = "Examples:\n  zatsu\n  zatsu --max-depth 2 .\n  zatsu --max-lines 200 .\n  zatsu src/lib.rs"
)]
struct Cli {
    /// File or directory to inspect
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,

    /// Limit directory traversal depth (root is 0)
    #[arg(short = 'd', long, value_name = "N")]
    max_depth: Option<usize>,

    /// Limit normal output lines (a truncation notice may follow)
    #[arg(
        short = 'l',
        long,
        value_name = "N",
        value_parser = parse_positive_usize
    )]
    max_lines: Option<usize>,
}

fn parse_positive_usize(value: &str) -> Result<usize, String> {
    let value = value
        .parse::<usize>()
        .map_err(|_| "must be a non-negative integer".to_owned())?;
    if value == 0 {
        return Err("must be at least 1".to_owned());
    }
    Ok(value)
}

fn print_outline(
    source: &str,
    ranges: &[outline::VisibleRange<'_>],
    prefix: &str,
    max_lines: Option<usize>,
) {
    let mut rendered = Vec::new();
    let _ = write_outline(source, ranges, prefix, &mut rendered);
    let rendered = String::from_utf8_lossy(&rendered);
    let mut out = std::io::stdout().lock();
    let mut lines = rendered.lines();

    for line in lines.by_ref().take(max_lines.unwrap_or(usize::MAX)) {
        use std::io::Write;
        let _ = writeln!(out, "{line}");
    }
    if let Some(max_lines) = max_lines
        && lines.next().is_some()
    {
        use std::io::Write;
        let _ = writeln!(out, "{}", truncation_message(max_lines));
    }
}

fn view_file(path: &Path, max_lines: Option<usize>) {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match lang_for_ext(ext) {
        Some((language, query_src)) => {
            let source = fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("zatsu: {}: {}", path.display(), e);
                std::process::exit(1);
            });
            let tree = match outline::parse(&source, &language) {
                Some(t) => t,
                None => return,
            };
            let ranges = outline::extract_outline(&source, &tree, &language, query_src);
            print_outline(&source, &ranges, "", max_lines);
        }
        None => {
            eprintln!("zatsu: {}: unknown file type", path.display());
            std::process::exit(1);
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let path = cli.path.as_path();
    if !path.exists() {
        eprintln!("zatsu: {}: No such file or directory", path.display());
        std::process::exit(1);
    }

    if path.is_dir() {
        let mut output = std::io::stdout().lock();
        let options = RepositoryOutlineOptions {
            max_depth: cli.max_depth,
            max_lines: cli.max_lines,
        };
        if let Err(error) = write_repository_outline_with_options(path, &options, &mut output) {
            eprintln!("zatsu: {}: {}", path.display(), error);
            std::process::exit(1);
        }
    } else {
        if cli.max_depth.is_some() {
            Cli::command()
                .error(
                    ErrorKind::ArgumentConflict,
                    "--max-depth can only be used with a directory",
                )
                .exit();
        }
        view_file(path, cli.max_lines);
    }
}
