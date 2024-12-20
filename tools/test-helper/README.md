# Test helper

This is a small VS Code extension that helps with managing Typst's test suite.
When installed, for all `.typ` files in the `tests` directory, the following
Code Lens buttons will appear above every test's name:

- View: Opens the output and reference image of a test to the side.
- Run: Runs the test and shows the results to the side.
- Save: Runs the test with `--update` to save the reference output.
- Terminal: Runs the test in the integrated terminal.

In the side panel opened by the Code Lens buttons, there are a few menu buttons
at the top right:

- Refresh: Reloads the panel to reflect changes to the images.
- Run: Runs the test and shows the results.
- Save: Runs the test with `--update` to save the reference output.

## Installation
In order for VS Code to run the extension with its built-in
[Node](https://nodejs.org) engine, you need to first build it from source.
Navigate to `test-helper` directory and build the extension:
```bash
npm install    # Install the dependencies.
npm run build  # Build the extension from source.
```

Then, you can easily install it (and keep it up-to-date) via VS Code's UI:
- Go to View > Command Palette or press Cmd/Ctrl+P,
- In the drop down list, pick command "Developer: Install Extension from
  Location",
- Select this `test-helper` directory in the file explorer dialogue box. VS Code
  will add the extension's path to `~/.vscode/extensions/extensions.json` (or
  `%USERPROFILE%\.vscode\extensions\extensions.json` on Windows).
