use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use zatsu::repository::write_repository_outline;

/// A copy of the checked-in fixture with a malformed binary file added.
///
/// Keeping the binary generated here avoids committing a non-text fixture while
/// still exercising the important property: repository traversal must not try
/// to parse invalid UTF-8 as source text.
struct RepositoryFixture {
    root: PathBuf,
}

impl RepositoryFixture {
    fn new() -> Self {
        let fixture = Path::new("tests/fixtures/repository");
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before the Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "zatsu-repository-fixture-{}-{suffix}",
            std::process::id()
        ));

        copy_tree(fixture, &root).expect("copy repository fixture");
        fs::create_dir_all(root.join("assets")).expect("create binary fixture directory");
        fs::write(
            root.join("assets/image.bin"),
            b"\x00\xff\x80ZAT_BINARY_MARKER\x00",
        )
        .expect("write binary fixture");
        fs::create_dir_all(root.join(".git")).expect("create VCS metadata fixture");
        fs::write(root.join(".git/config"), "VCS_METADATA_MARKER")
            .expect("write VCS metadata fixture");

        Self { root }
    }
}

impl Drop for RepositoryFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn copy_tree(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_tree(&source_path, &destination_path)?;
        } else {
            fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}

fn render_repository(fixture: &RepositoryFixture) -> String {
    let mut output = Vec::new();
    write_repository_outline(&fixture.root, &mut output).expect("write repository outline");
    String::from_utf8(output).expect("repository outline should be UTF-8")
}

#[test]
fn repository_outline_contains_tree_and_supported_file_symbols() {
    let fixture = RepositoryFixture::new();
    let output = render_repository(&fixture);
    let body = output.lines().skip(1).collect::<Vec<_>>().join("\n");

    assert_eq!(
        body,
        "├── assets/\n\
         │   └── image.bin\n\
         ├── docs/\n\
         │   └── notes.txt\n\
         ├── src/\n\
         │   ├── lib.rs\n\
         │   │   └── fn visible_api() -> &'static str // L1-L3\n\
         │   └── nested.py\n\
         │       └── def nested_api(value: int) -> int: // L1-L2\n\
         └── .gitignore"
    );
}

#[test]
fn repository_outline_respects_gitignore() {
    let fixture = RepositoryFixture::new();
    let output = render_repository(&fixture);

    assert!(
        !output.contains("ignored/")
            && !output.contains("secret.rs")
            && !output.contains("should_not_appear"),
        "gitignored source and its symbols must be absent:\n{output}"
    );
}

#[test]
fn repository_outline_skips_unsupported_and_binary_content_safely() {
    let fixture = RepositoryFixture::new();
    let output = render_repository(&fixture);

    assert!(
        !output.contains("fake_unsupported_symbol"),
        "unsupported files must not be parsed as source:\n{output}"
    );
    assert!(
        !output.contains("ZAT_BINARY_MARKER"),
        "binary contents must not be emitted or parsed:\n{output}"
    );
    assert!(
        !output.contains(".git/") && !output.contains("VCS_METADATA_MARKER"),
        "VCS metadata must be omitted:\n{output}"
    );
}
