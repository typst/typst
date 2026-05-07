# Documentation

This repository contains the sources for the official documentation that's available on https://typst.app/docs. The documentation can be built in two modes: As a website and as a standalone PDF.

## Directory structure
- `src`: Rust sources that support the documentation infrastructure.
- `content`: Typst sources containing documentation content. Supplemented by Typst markup in Rust doc comments throughout the codebase. Note that not all doc comments are Typst markup, only those affected by the `#[elem]`, `#[ty]`, and `#[func]` macros.
- `components`: Typst sources that make up the infrastructure of the documentation.
- `assets`: Static assets that will be emitted into the resulting website. Assets that are used by the docs content itself don't live there. These are in the `typst-dev-assets` repository.
- `dev`: Developer-facing documentation (for the codebase).

## Commands
This repository includes the alias `cargo docit` to interface with the docs infrastructure.

### Building the website version
```bash
cargo docit compile
```
The resulting site will be placed into `docs/dist/site`.

### Building the PDF version
```bash
cargo docit compile --format pdf
```
The resulting PDF will be placed into `docs/dist/docs.pdf`.

### Watch mode for docs writing
For local docs writing, the documentation can be served in watch mode, with hot reload. Hot reload that affects both Typst sources in `docs/` and documentation in doc comments in the Rust sources in `crates/`.
```bash
cargo docit watch
```

Similarly, to the `compile` subcommand, the `watch` command supports both the website and PDF output.

### Further options
There are a few more options you can set from the CLI. You can learn more about them by running `cargo docit help`.
