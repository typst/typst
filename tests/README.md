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

## Update expected images
If you created a new test or fixed a bug in an existing test, you need to update
the reference image used for comparison. For this, you can use the
`UPDATE_EXPECT` environment variable or the `--update` flag:
```bash
testit mytest --update
```

If you use the VS Code test helper extension (see the `tools` folder), you can
alternatively use the checkmark button to update the reference image. In that
case you should also install `oxipng` on your system so that the test helper
can optimize the reference images.

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
