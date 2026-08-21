use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_zatsu"))
        .args(args)
        .output()
        .expect("run zatsu")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

#[test]
fn help_is_available_with_short_and_long_flags() {
    for flag in ["-h", "--help"] {
        let output = run(&[flag]);
        assert!(output.status.success(), "{}", stderr(&output));

        let help = stdout(&output);
        assert!(help.contains("Usage: zatsu [OPTIONS] [PATH]"), "{help}");
        assert!(help.contains("-h, --help"), "{help}");
        assert!(help.contains("-V, --version"), "{help}");
        assert!(help.contains("-d, --max-depth <N>"), "{help}");
        assert!(help.contains("-l, --max-lines <N>"), "{help}");
    }
}

#[test]
fn version_is_available_with_short_and_long_flags() {
    let expected = format!("zatsu {}\n", env!("CARGO_PKG_VERSION"));

    for flag in ["-V", "--version"] {
        let output = run(&[flag]);
        assert!(output.status.success(), "{}", stderr(&output));
        assert_eq!(stdout(&output), expected);
    }
}

#[test]
fn no_arguments_inspects_the_current_directory() {
    let output = Command::new(env!("CARGO_BIN_EXE_zatsu"))
        .current_dir("tests/fixtures/repository")
        .output()
        .expect("run zatsu");

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).starts_with("repository/\n"));
}

#[test]
fn unknown_options_are_rejected() {
    let output = run(&["--unknown-option"]);

    assert_eq!(output.status.code(), Some(2));
    let error = stderr(&output);
    assert!(
        error.contains("unexpected argument '--unknown-option'"),
        "{error}"
    );
    assert!(error.contains("--help"), "{error}");
}

#[test]
fn double_dash_allows_paths_that_start_with_a_hyphen() {
    let output = Command::new(env!("CARGO_BIN_EXE_zatsu"))
        .current_dir("tests/fixtures")
        .args(["--", "-sample.rs"])
        .output()
        .expect("run zatsu");

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("fn hyphenated_path()"));
}

#[test]
fn max_depth_limits_directory_output() {
    let output = run(&["--max-depth", "1", "tests/fixtures/repository"]);

    assert!(output.status.success(), "{}", stderr(&output));
    let output = stdout(&output);
    assert!(output.contains("src/"), "{output}");
    assert!(output.contains("… (max depth reached)"), "{output}");
    assert!(!output.contains("nested.py"), "{output}");
}

#[test]
fn max_depth_is_rejected_in_file_mode() {
    let output = run(&["--max-depth", "1", "src/lib.rs"]);

    assert_eq!(output.status.code(), Some(2));
    let error = stderr(&output);
    assert!(
        error.contains("--max-depth can only be used with a directory"),
        "{error}"
    );
}

#[test]
fn max_lines_limits_file_output() {
    let output = run(&["--max-lines", "1", "tests/fixtures/sample.rs"]);

    assert!(output.status.success(), "{}", stderr(&output));
    let output = stdout(&output);
    assert_eq!(output.lines().count(), 2, "{output}");
    assert!(
        output.ends_with("… (output truncated after 1 line)\n"),
        "{output}"
    );
}

#[test]
fn max_lines_must_be_positive() {
    let output = run(&["--max-lines", "0"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("must be at least 1"));
}
