# Test helper

This is a small VS Code extension that helps with managing Typst's test suite.
When installed, three new buttons appear in the menubar for all `.typ` files in
the `tests` folder.

- Open: Opens the output and reference images of a test to the side.
- Refresh: Refreshes the preview.
- Rerun: Re-runs the test.
- Update: Copies the output into the reference folder and optimizes
  it with `oxipng`.

For the test helper to work correctly, you also need to install `oxipng`, for
example with `cargo install oxipng`.

## Installation
The simplest way to install this extension (and keep it up-to-date) is to add a
symlink from `~/.vscode/extensions/typst-test-helper` to
`path/to/typst/tools/test-helper`.
