# Tests

## Directory structure
Top level directory structure:
- `src`: Testing code.
- `typ`: Input files. The tests in `compiler` specifically test the compiler
         while the others test the standard library (but also the compiler
         indirectly).
- `ref`: Reference images which the output is compared with to determine whether
         a test passed or failed.
- `png`: PNG files produced by tests.
- `pdf`: PDF files produced by tests.

## Running the tests
Running all tests (including unit tests):
```bash
cargo test --all
```

Running just the integration tests (the tests in this directory):
```bash
cargo test --all --test tests
```

You may want to [make yourself an alias](#making-an-alias) like:
```bash
testit
```

Running all tests whose paths contain the string `page` or `stack`.
```bash
testit page stack
```

Running a test with the exact filename `page.typ`.
```bash
testit --exact page.typ
```

Debug-printing the layout trees for all executed tests.
```bash
testit --debug empty.typ
```

To make the integration tests go faster they don't generate PDFs by default.
Pass the `--pdf` flag to generate those. Mind that PDFs are not tested
automatically at the moment, so you should always check the output manually when
making changes.
```bash
testit --pdf
```

## Creating new tests
To keep things small, please optimize reference images before committing them.
When you use the approve button from the Test Helper (see the `tools` folder)
this happens automatically if you have `oxipng` installed.
```bash
# One image
oxipng -o max path/to/image.png

# All images
oxipng -r -o max tests/ref
```

## Making an alias
If you want to have a quicker way to run the tests, consider adding a shortcut
to your shell profile so that you can simply write something like:
```bash
testit empty.typ
```

### Bash
Open your Bash configuration by executing `nano ~/.bashrc`.
```bash
alias testit="cargo test --all --test tests --"
```

### PowerShell
Open your PowerShell profile by executing `notepad $profile`.
```ps
function testit {
    cargo test --all --test tests -- $args
}
```
