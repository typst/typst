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

Running all tests whose names contain the word `filter`.
```bash
cargo test --test typeset filter
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
