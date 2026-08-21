# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working in
this repository.

## Project overview

`zatsu` is a Rust CLI for deterministic repository and file outlines. Directory
mode prints a `.gitignore`-aware file tree and symbol outlines for supported
source files. File mode prints the outline for one file.

## Build and test

```bash
cargo build                  # Dev build
cargo build --release        # Release build
nix build                    # Build via Nix
cargo test                   # Run all tests
```

## Release

Tagging a `v*` version triggers `.github/workflows/release.yml`, which builds
macOS and Linux binaries and publishes a GitHub Release for this repository.

## Architecture

- `src/main.rs`: CLI entry point and file/directory dispatch.
- `src/repository.rs`: `.gitignore`-aware traversal and repository tree output.
- `src/outline.rs`: tree-sitter outline extraction and rendering.
- `queries/*.scm`: language-specific captures that define visible symbols.
- `src/lib.rs`: extension-to-language registry.
- `flake.nix`: Nix build and Home Manager module exports.

## Adding a language

1. Add the tree-sitter dependency to `Cargo.toml`.
2. Add a query under `queries/`.
3. Register extensions in `src/lib.rs`.
4. Add fixtures and snapshots under `tests/`.
5. Add or update the corresponding test.

This project is a GPL-3.0 fork of [bglgwyng/zat](https://github.com/bglgwyng/zat);
retain the upstream credit when adapting its outline engine or queries.
