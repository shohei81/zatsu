use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use ignore::{DirEntry, WalkBuilder};

use crate::lang_for_ext;
use crate::outline::{extract_outline, parse, write_outline};

#[derive(Default)]
struct Directory {
    directories: BTreeMap<String, Directory>,
    files: BTreeSet<String>,
}

impl Directory {
    fn insert_directory(&mut self, relative_path: &Path) {
        let mut directory = self;

        for component in relative_path.components() {
            let name = component.as_os_str().to_string_lossy().into_owned();
            directory = directory.directories.entry(name).or_default();
        }
    }

    fn insert_file(&mut self, relative_path: &Path) {
        let mut components = relative_path.components().peekable();
        let mut directory = self;

        while let Some(component) = components.next() {
            let name = component.as_os_str().to_string_lossy().into_owned();
            if components.peek().is_none() {
                directory.files.insert(name);
            } else {
                directory = directory.directories.entry(name).or_default();
            }
        }
    }
}

/// Write a deterministic, repository-level view of `root`.
///
/// Files ignored by Git are omitted. Every remaining file is shown in the tree;
/// supported source files also include the same symbol outline as file mode.
pub fn write_repository_outline(root: &Path, writer: &mut impl Write) -> io::Result<()> {
    let mut tree = Directory::default();
    let mut source_paths = BTreeMap::new();

    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .require_git(false)
        .parents(true)
        .filter_entry(is_not_vcs_metadata);

    for result in builder.build() {
        let entry = result.map_err(ignore_error_to_io)?;
        let relative_path = entry
            .path()
            .strip_prefix(root)
            .map_err(|error| io::Error::other(error.to_string()))?;
        if relative_path.as_os_str().is_empty() {
            continue;
        }

        if entry
            .file_type()
            .is_some_and(|file_type| file_type.is_dir())
        {
            tree.insert_directory(relative_path);
            continue;
        }
        if !entry
            .file_type()
            .is_some_and(|file_type| file_type.is_file())
        {
            continue;
        }

        tree.insert_file(relative_path);
        if is_supported_source(relative_path) {
            source_paths.insert(path_key(relative_path), entry.into_path());
        }
    }

    writeln!(writer, "{}/", root_name(root))?;
    write_directory(&tree, Path::new(""), &source_paths, "", writer)
}

fn write_directory(
    directory: &Directory,
    relative_path: &Path,
    source_paths: &BTreeMap<String, PathBuf>,
    prefix: &str,
    writer: &mut impl Write,
) -> io::Result<()> {
    let child_count = directory.directories.len() + directory.files.len();
    let mut child_index = 0;

    for (name, child) in &directory.directories {
        child_index += 1;
        let is_last = child_index == child_count;
        writeln!(writer, "{prefix}{}{name}/", connector(is_last))?;

        let child_path = relative_path.join(name);
        let child_prefix = format!("{prefix}{}", continuation(is_last));
        write_directory(child, &child_path, source_paths, &child_prefix, writer)?;
    }

    for name in &directory.files {
        child_index += 1;
        let is_last = child_index == child_count;
        writeln!(writer, "{prefix}{}{name}", connector(is_last))?;

        let file_path = relative_path.join(name);
        if let Some(path) = source_paths.get(&path_key(&file_path))
            && let Some(outline) = outline_for_file(path)
        {
            write_outline_lines(&outline, prefix, is_last, writer)?;
        }
    }

    Ok(())
}

fn write_outline_lines(
    outline: &str,
    prefix: &str,
    file_is_last: bool,
    writer: &mut impl Write,
) -> io::Result<()> {
    let lines: Vec<_> = outline.lines().filter(|line| !line.is_empty()).collect();
    let outline_prefix = format!("{prefix}{}", continuation(file_is_last));

    for (index, line) in lines.iter().enumerate() {
        let is_last = index + 1 == lines.len();
        writeln!(writer, "{outline_prefix}{}{line}", connector(is_last))?;
    }

    Ok(())
}

fn outline_for_file(path: &Path) -> Option<String> {
    let extension = path.extension().and_then(OsStr::to_str).unwrap_or("");
    let (language, query_source) = lang_for_ext(extension)?;
    let source = fs::read_to_string(path).ok()?;
    let tree = parse(&source, &language)?;
    let ranges = extract_outline(&source, &tree, &language, query_source);
    let mut output = Vec::new();
    write_outline(&source, &ranges, "", &mut output).ok()?;
    String::from_utf8(output).ok()
}

fn is_supported_source(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| lang_for_ext(extension).is_some())
}

fn is_not_vcs_metadata(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return true;
    }

    !entry
        .file_type()
        .is_some_and(|file_type| file_type.is_dir())
        || !matches!(entry.file_name().to_str(), Some(".git" | ".hg" | ".svn"))
}

fn root_name(root: &Path) -> String {
    root.canonicalize()
        .ok()
        .as_deref()
        .and_then(Path::file_name)
        .or_else(|| root.file_name())
        .unwrap_or_else(|| OsStr::new("."))
        .to_string_lossy()
        .into_owned()
}

fn path_key(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn connector(is_last: bool) -> &'static str {
    if is_last { "└── " } else { "├── " }
}

fn continuation(is_last: bool) -> &'static str {
    if is_last { "    " } else { "│   " }
}

fn ignore_error_to_io(error: ignore::Error) -> io::Error {
    io::Error::other(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_inserts_nested_files() {
        let mut directory = Directory::default();
        directory.insert_file(Path::new("src/parser.rs"));

        assert!(directory.directories["src"].files.contains("parser.rs"));
    }

    #[test]
    fn directory_inserts_empty_directories() {
        let mut directory = Directory::default();
        directory.insert_directory(Path::new("src/generated"));

        assert!(
            directory.directories["src"]
                .directories
                .contains_key("generated")
        );
    }
}
