# Tests

## Directory structure
Top level directory structure:
- `typ`: Input files.
- `res`: Resource files used by tests.
- `ref`: Reference images which the output is compared with to determine whether
         a test passed or failed.
- `png`: PNG files produced by tests.
- `pdf`: PDF files produced by tests.

## Running the tests
Running the integration tests (the tests in this directory).
```bash
cargo test --test typeset
```

Running all tests whose paths contain the string `page` or `stack`.
```bash
cargo test --test typeset page stack
```

Running a test with the exact filename `page.typ`.
```bash
cargo test --test typeset -- --exact page.typ
```

Debug-printing the layout trees for all executed tests.
```bash
cargo test --test typeset -- --debug empty.typ
```

To make the integration tests go faster they don't generate PDFs by default.
Pass the `--pdf` flag to generate those. Mind that PDFs are not tested
automatically at the moment, so you should always check the output manually when
making changes.
```bash
cargo test --test typeset -- --pdf
```

## Creating new tests
To keep things small, please optimize reference images before committing them.
When you use the approve buttom from the Test Helper (see the `tools` folder)
this happens automatically if you have `oxipng` installed.
```bash
# One image
oxipng -o max path/to/image.png

# All images
oxipng -r -o max tests/ref
```

## Shorthand for running tests
If you want to have a quicker way to run the tests, consider adding a shortcut
to your shell profile so that you can simply write something like:
```bash
tests --debug empty.typ
```

### PowerShell
Open your PowerShell profile by executing `notepad $profile`.
```ps
function tests {
    cargo test --test typeset -- $args
}
```

### Bash
Open your Bash configuration by executing `nano ~/.bashrc`.
```bash
alias tests="cargo test --test typeset --"
```
