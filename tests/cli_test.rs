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
        assert!(help.contains("Usage: zatsu [PATH]"), "{help}");
        assert!(help.contains("-h, --help"), "{help}");
        assert!(help.contains("-V, --version"), "{help}");
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
