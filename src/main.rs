mod outline;

use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use outline::write_outline;
use zatsu::lang_for_ext;
use zatsu::repository::write_repository_outline;

#[derive(Parser)]
#[command(
    version,
    about = "See a repository or source file at a glance",
    after_help = "Examples:\n  zatsu\n  zatsu .\n  zatsu src/lib.rs"
)]
struct Cli {
    /// File or directory to inspect
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,
}

fn print_outline(source: &str, ranges: &[outline::VisibleRange<'_>], prefix: &str) {
    let mut out = std::io::stdout().lock();
    let _ = write_outline(source, ranges, prefix, &mut out);
}

fn view_file(path: &Path) {
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
            print_outline(&source, &ranges, "");
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
        if let Err(error) = write_repository_outline(path, &mut output) {
            eprintln!("zatsu: {}: {}", path.display(), error);
            std::process::exit(1);
        }
    } else {
        view_file(path);
    }
}
