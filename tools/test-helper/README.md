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
example with `cargo install oxipng`. Make sure that the version of oxipng you
install is the same as the one in the root `Cargo.toml` so that the results are
the same as when using the test CLI.

## Installation
The simplest way to install this extension (and keep it up-to-date) is to use VSCode's UI:
* Go to View > Command Palette,
* In the drop down list, pick command "Developer: Install extension from location",
* Select this `test-helper` directory in the file explorer dialogue box. VSCode will add
the extension's path to `~/.vscode/extensions/extensions.json`.
