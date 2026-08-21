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
    truncated: bool,
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

    fn mark_truncated(&mut self, relative_path: &Path) {
        let mut directory = self;

        for component in relative_path.components() {
            let name = component.as_os_str().to_string_lossy().into_owned();
            directory = directory.directories.entry(name).or_default();
        }

        directory.truncated = true;
    }
}

/// Limits applied while rendering a repository outline.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RepositoryOutlineOptions {
    /// Maximum tree depth, where the repository root is depth 0.
    pub max_depth: Option<usize>,
    /// Maximum number of normal output lines, excluding a truncation notice.
    pub max_lines: Option<usize>,
}

struct RenderState {
    max_lines: Option<usize>,
    lines_written: usize,
    truncated: bool,
}

impl RenderState {
    fn new(max_lines: Option<usize>) -> Self {
        Self {
            max_lines,
            lines_written: 0,
            truncated: false,
        }
    }

    fn write_line(&mut self, line: &str, writer: &mut impl Write) -> io::Result<bool> {
        if self
            .max_lines
            .is_some_and(|max_lines| self.lines_written >= max_lines)
        {
            self.truncated = true;
            return Ok(false);
        }

        writeln!(writer, "{line}")?;
        self.lines_written += 1;
        Ok(true)
    }
}

/// Write a deterministic, repository-level view of `root`.
///
/// Files ignored by Git are omitted. Every remaining file is shown in the tree;
/// supported source files also include the same symbol outline as file mode.
pub fn write_repository_outline(root: &Path, writer: &mut impl Write) -> io::Result<()> {
    write_repository_outline_with_options(root, &RepositoryOutlineOptions::default(), writer)
}

/// Write a deterministic, repository-level view of `root` with output limits.
pub fn write_repository_outline_with_options(
    root: &Path,
    options: &RepositoryOutlineOptions,
    writer: &mut impl Write,
) -> io::Result<()> {
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
    builder.max_depth(options.max_depth.map(|depth| {
        if depth == 0 {
            0
        } else {
            depth.saturating_add(1)
        }
    }));

    for result in builder.build() {
        let entry = result.map_err(ignore_error_to_io)?;
        let relative_path = entry
            .path()
            .strip_prefix(root)
            .map_err(|error| io::Error::other(error.to_string()))?;
        if relative_path.as_os_str().is_empty() {
            continue;
        }

        if options
            .max_depth
            .is_some_and(|max_depth| entry.depth() > max_depth)
        {
            if entry
                .file_type()
                .is_some_and(|file_type| file_type.is_dir() || file_type.is_file())
                && let Some(parent) = relative_path.parent()
            {
                tree.mark_truncated(parent);
            }
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

    let mut state = RenderState::new(options.max_lines);
    if state.write_line(&format!("{}/", root_name(root)), writer)? {
        write_directory(&tree, Path::new(""), &source_paths, "", &mut state, writer)?;
    }
    if state.truncated {
        writeln!(
            writer,
            "{}",
            truncation_message(options.max_lines.unwrap_or_default())
        )?;
    }
    Ok(())
}

fn write_directory(
    directory: &Directory,
    relative_path: &Path,
    source_paths: &BTreeMap<String, PathBuf>,
    prefix: &str,
    state: &mut RenderState,
    writer: &mut impl Write,
) -> io::Result<()> {
    let child_count =
        directory.directories.len() + directory.files.len() + usize::from(directory.truncated);
    let mut child_index = 0;

    for (name, child) in &directory.directories {
        child_index += 1;
        let is_last = child_index == child_count;
        if !state.write_line(&format!("{prefix}{}{name}/", connector(is_last)), writer)? {
            return Ok(());
        }

        let child_path = relative_path.join(name);
        let child_prefix = format!("{prefix}{}", continuation(is_last));
        write_directory(
            child,
            &child_path,
            source_paths,
            &child_prefix,
            state,
            writer,
        )?;
        if state.truncated {
            return Ok(());
        }
    }

    for name in &directory.files {
        child_index += 1;
        let is_last = child_index == child_count;
        if !state.write_line(&format!("{prefix}{}{name}", connector(is_last)), writer)? {
            return Ok(());
        }

        let file_path = relative_path.join(name);
        if let Some(path) = source_paths.get(&path_key(&file_path))
            && let Some(outline) = outline_for_file(path)
        {
            write_outline_lines(&outline, prefix, is_last, state, writer)?;
            if state.truncated {
                return Ok(());
            }
        }
    }

    if directory.truncated {
        state.write_line(
            &format!(
                "{prefix}{}… (max depth reached)",
                connector(child_index + 1 == child_count)
            ),
            writer,
        )?;
    }

    Ok(())
}

fn write_outline_lines(
    outline: &str,
    prefix: &str,
    file_is_last: bool,
    state: &mut RenderState,
    writer: &mut impl Write,
) -> io::Result<()> {
    let lines: Vec<_> = outline.lines().filter(|line| !line.is_empty()).collect();
    let outline_prefix = format!("{prefix}{}", continuation(file_is_last));

    for (index, line) in lines.iter().enumerate() {
        let is_last = index + 1 == lines.len();
        if !state.write_line(
            &format!("{outline_prefix}{}{line}", connector(is_last)),
            writer,
        )? {
            break;
        }
    }

    Ok(())
}

pub fn truncation_message(max_lines: usize) -> String {
    let unit = if max_lines == 1 { "line" } else { "lines" };
    format!("… (output truncated after {max_lines} {unit})")
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
