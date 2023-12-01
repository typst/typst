# Test helper

This is a small VS Code extension that helps with managing Typst's test suite.
When installed, a few new buttons appear in the menubar for all `.typ` files in
the `tests` folder.

- Open: Opens the output and reference images of a test to the side.
- Refresh: Refreshes the preview.
- Rerun: Re-runs the test.
- Update: Update the reference image.

Under the hood, the extension uses the [same CLI command](../../tests/README.md) to run
the tests and to update the reference images.

## Installation
The simplest way to install this extension (and keep it up-to-date) is to use VSCode's UI:
* Go to View > Command Palette,
* In the drop down list, pick command "Developer: Install extension from location",
* Select this `test-helper` directory in the file explorer dialogue box. VSCode will add
the extension's path to `~/.vscode/extensions/extensions.json`.
