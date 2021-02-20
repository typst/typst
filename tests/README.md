# Tests

## Directory structure
Top level directory structure:
- `typ`: Input files.
- `ref`: Reference images which the output is compared with to determine whether
         a test passed or failed.
- `png`: PNG files produced by tests.
- `pdf`: PDF files produced by tests.
- `res`: Resource files used by tests.

## Running the tests
```bash
# Run all tests
cargo test

# Run unit tests
cargo test --lib

# Run integration tests (the tests in this directory)
cargo test --test typeset

# Run all tests whose names contain the word `filter`
cargo test --test typeset filter
```

## Creating new tests
To keep things small, please optimize reference images before committing them:
```bash
# One image
oxipng -o max path/to/image.png

# All images
oxipng -r -o max tests/ref
```
