# Tests

## Directory structure
Top level directory structure:
- `src`: Testing code.
- `suite`: Input files. Mostly organized in parallel to the code. Each file can
           contain multiple tests, each of which is a section of Typst code
           following `--- {name} ---`.
- `ref`: References which the output is compared with to determine whether a
         test passed or failed.
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

The repository includes the alias `cargo testit` to make this less verbose. In
the examples below, we will use this alias.

Running all tests with the given name pattern. You can use
[regular expression](https://docs.rs/regex/latest/regex/)s.
```bash
cargo testit math            # The name has "math" anywhere
cargo testit math page       # The name has "math" or "page" anywhere
cargo testit "^math" "^page" # The name begins with "math" or "page"
cargo testit "^(math|page)"  # Same as above.
```

Running all tests discovered under given paths:
```bash
cargo testit -p tests/suite/math/attach.typ
cargo testit -p tests/suite/model -p tests/suite/text
```

Running tests that begin with `issue` under a given path:
```bash
cargo testit "^issue" -p tests/suite/model
```

Running a test with the exact test name `math-attach-mixed`.
```bash
cargo testit --exact math-attach-mixed
```

You may find more options in the help message:
```bash
cargo testit --help
```

To make the integration tests go faster they don't generate PDFs or SVGs by
default. Pass the `--pdf` or `--svg` flag to generate those. Mind that PDFs and
SVGs are **not** tested automatically at the moment, so you should always check
the output manually when making changes.
```bash
cargo testit --pdf
```

## Writing tests
The syntax for an individual test is `--- {name} {attr}* ---` followed by some
Typst code that should be tested. The name must be globally unique in the test
suite, so that tests can be easily migrated across files. A test name can be
followed by space-separated attributes. For instance, `--- my-test html ---`
adds the `html` modifier to `my-test`, instructing the test runner to also
test HTML output. The following attributes are currently defined:

- `render`: Tests paged output against a reference image (the default, only
  needs to be specified when `html` is also specified to enable both at the
  same)
- `html`: Tests HTML output against a reference HTML file. Disables the `render`
  default.
- `large`: Permits a reference image size exceeding 20 KiB. Should be used
  sparingly.

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

- Tests that ensure certain output is produced:

  - Visual output: By default, the compiler produces paged output, renders it
    with the `typst-render` crate, and compares it against a reference image
    stored in the repository. The test runner automatically detects whether a
    test has visual output and requires a reference image in this case.

    To prevent bloat, it is important that the test images are kept as small as
    possible. To that effect, the test runner enforces a maximum size of 20 KiB.
    If you're updating a test and hit `reference output size exceeds`, see the
    section on "Updating reference images" below. If truly necessary, the size
    limit can be lifted by adding a `large` attribute after the test name, but
    this should be the case very rarely.

  - HTML output: When a test has the `html` attribute, the compiler produces
    HTML output and compares it against a reference file stored in the
    repository. By default, this enables testing of paged output, but you can
    test both at once by passing both `render` and `html` as attributes.

If you have the choice between writing a test using assertions or using
reference images, prefer assertions. This makes the test easier to understand
in isolation and prevents bloat due to images.

## Updating reference images
If you created a new test or fixed a bug in an existing test, you may need to
update the reference output used for comparison. For this, you can use the
`--update` flag:
```bash
cargo testit --exact my-test-name --update
```

For visual tests, this will generally generate compressed reference images (to
remain within the size limit).

If you use the VS Code test helper extension (see the `tools` folder), you can
alternatively use the save button to update the reference output.
