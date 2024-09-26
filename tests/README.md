# Tests

## Directory structure
Top level directory structure:
- `src`: Testing code.
- `suite`: Input files. Mostly organized in parallel to the code. Each file can
           contain multiple tests, each of which is a section of Typst code
           following `--- {name} ---`.
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

You may want to [make yourself an alias](#making-an-alias) `testit` so that you can
write shorter commands. In the examples below, we will use this alias.

Running all tests with the given name pattern. You can use
[regular expression](https://docs.rs/regex/latest/regex/)s.
```bash
testit math            # The name has "math" anywhere
testit math page       # The name has "math" or "page" anywhere
testit "^math" "^page" # The name begins with "math" or "page"
testit "^(math|page)"  # Same as above.
```

Running all tests discovered under given paths:
```bash
testit -p tests/suite/math/attach.typ
testit -p tests/suite/model -p tests/suite/text
```

Running tests that begin with `issue` under a given path:
```bash
testit "^issue" -p tests/suite/model
```

Running a test with the exact test name `math-attach-mixed`.
```bash
testit --exact math-attach-mixed
```

You may find more options in the help message:
```bash
testit --help
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

- Tests that ensure the code emits particular diagnostic messages: Those have
  inline annotations like `// Error: 2-7 thing was wrong`. An annotation can
  start with either "Error", "Warning", or "Hint". The range designates the
  code span the diagnostic message refers to in the first non-comment line
  below. If the code span is in a line further below, you can write ranges
  like `3:2-3:7` to indicate the 2-7 column in the 3rd non-comment line.

- Tests that ensure certain visual output is produced: Those render the result
  of the test with the `typst-render` crate and compare against a reference
  image stored in the repository. The test runner automatically detects whether
  a test has visual output and requires a reference image in this case.

  To prevent bloat, it is important that the test images are kept as small as
  possible. To that effect, the test runner enforces a maximum size of 20 KiB.
  If you're updating a test and hit `reference image size exceeds`, see
  Updating reference images.
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
testit --exact my-test-name --update
```

This will generally generate compressed reference images (to remain within the
above size limit).

If you use the VS Code test helper extension (see the `tools` folder), you can
alternatively use the save button to update the reference image.

## Making an alias
If you want to have a quicker way to run the tests, consider adding a shortcut
to your shell profile so that you can simply write something like:
```bash
testit --exact my-test-name
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
