mod outline;

use std::fs;
use std::path::Path;

use outline::write_outline;
use zatsu::lang_for_ext;
use zatsu::repository::write_repository_outline;

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
    let path_arg = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: zatsu <path>");
        std::process::exit(1);
    });

    let path = Path::new(&path_arg);
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
