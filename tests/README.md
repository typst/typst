# Tests

## Directory structure
Top level directory structure:
- `src`: Testing code.
- `suite`: Input files. Mostly organize in parallel to the code.
- `ref`: Reference images which the output is compared with to determine whether
         a test passed or failed.
- `store`: Store for PNG, PDF, and SVG output files produced by the tests.

## Running the tests
Running all tests (including unit tests):
```bash
cargo test --workspace
```

Running just the integration tests (the tests in this directory):
```bash
cargo test --workspace --test tests
```

You may want to [make yourself an alias](#making-an-alias) like:
```bash
testit
```

Running all tests whose names contain the string `page` or `stack`. Note each
`.typ` file in this directory can contain multiple tests, each of which is a
section of Typst code following `--- {name} ---`.
```bash
# Add --verbose to list which tests were run.
testit page stack
```

Running a test with the exact test name `math-attach-mixed`.
```bash
testit --exact math-attach-mixed
```

To make the integration tests go faster they don't generate PDFs by default.
Pass the `--pdf` flag to generate those. Mind that PDFs are not tested
automatically at the moment, so you should always check the output manually when
making changes.
```bash
testit --pdf
```

## Writing tests
The syntax for an individual test is `--- {name} ---` followed by some Typst
code that should be tested. The name must be globally unique in the test suite,
so that tests can be easily migrated across files.

There are, broadly speaking, three kinds of tests:

- Tests that just ensure that the code runs successfully: Those typically make
  use of `test` or `assert.eq` (both are very similar, `test` is just shorter)
  to ensure certain properties hold when executing the Typst code.

- Tests that ensure the code fails with a particular error: Those have inline
  annotations like `// Error: 2-7 thing was wrong`. An annotation can be
  either an "Error", a "Warning", or a "Hint". The range designates where
  in the next non-comment line the error is and after it follows the message.
  If you the error is in a line further below, you can also write ranges like
  `3:2-3:7` to indicate the 2-7 column in the 3rd non-comment line.

- Tests that ensure certain visual output is produced: Those render the result
  of the test with the `typst-render` crate and compare against a reference
  image stored in the repository. The test runner automatically detects whether
  a test has visual output and requires a reference image in this case.

  To prevent bloat, it is important that the test images are kept as small as
  possible. To that effect, the test runner enforces a maximum size of 20 KiB.
  If truly necessary, this limit can however be lifted by adding `// LARGE` as
  the first line of a test.

If you have the choice between writing a test using assertions or using
reference images, prefer assertions. This makes the test easier to understand
in isolation and prevents bloat due to images.

## Updating reference images
If you created a new test or fixed a bug in an existing test, you need to update
the reference image used for comparison. For this, you can use the `--update`
flag:
```bash
testit mytest --update
```

If you use the VS Code test helper extension (see the `tools` folder), you can
alternatively use the save button to update the reference image.

## Making an alias
If you want to have a quicker way to run the tests, consider adding a shortcut
to your shell profile so that you can simply write something like:
```bash
testit empty.typ
```

### Bash
Open your Bash configuration by executing `nano ~/.bashrc`.
```bash
alias testit="cargo test --workspace --test tests --"
```

### PowerShell
Open your PowerShell profile by executing `notepad $profile`.
```ps
function testit {
    cargo test --workspace --test tests -- $args
}
```
