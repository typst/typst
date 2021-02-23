# Test helper

This is a small VS Code extension that helps with managing Typst's test suite.
When installed, three new buttons appear in the menubar for all `.typ` files in
the `tests` folder.

- Open test output: Opens the output and reference images of a test to the side.
- Refresh test output: Re-runs the test and reloads the preview.
- Approve test output: Copies the output into the reference folder and optimizes
  it with `oxipng`.
