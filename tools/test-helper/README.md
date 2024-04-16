# Test helper

This is a small VS Code extension that helps with managing Typst's test suite.
When installed, a new Code Lens appears in all `.typ` files in the `tests`
folder. It provides the following actions:

- View: Opens the output and reference image of a test to the side.
- Run: Runs the test and shows the results to the side.
- Terminal: Runs the test in the integrated terminal.

In the side panel, there are a few menu actions at the top right:

- Refresh: Reloads the panel to reflect changes to the images
- Run: Runs the test and shows the results
- Save: Runs the test with `--update` to save the reference image

## Installation
First, you need to build the extension:
```bash
npm i
npm run build
```

Then, you can easily install and (and keep it up-to-date) via VS Code's UI:
- Go to View > Command Palette or press Cmd/Ctrl+P,
- In the drop down list, pick command "Developer: Install extension from
  location",
- Select this `test-helper` directory in the file explorer dialogue box. VS Code
  will add the extension's path to `~/.vscode/extensions/extensions.json`.
