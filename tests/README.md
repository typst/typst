# Tests

## Running the tests
```bash
# Run all tests
cargo test

# Run unit tests
cargo test --lib

# Run integration tests (the tests in this directory)
cargo test --test typeset

# Run all tests whose names contain a filter word
cargo test --test typeset call
```

For experimenting it's often useful to have a test file you can quickly run. For that purpose you can have a file named `playground.typ` right in this directory (the file is ignored by git). The playground test will be executed whenever no other test matches the filter, so you can run it with (since no real test's name contains an underscore):
```bash
cargo test --test typeset _
```

## Directory structure
Top level directory structure:
- `full`: Tests of full documents.
- `lang`: Tests for specific language features.
- `library`: Tests for specific library functions.
- `res`: Resource files used by tests.

Directory structure for each category:
- `typ`: Input files.
- `ref`: Reference images which the output is compared with to determine whether
         a test passed or failed.
- `png`: PNG files produced by tests.
- `pdf`: PDF files produced by tests.

To keep things small, please optimize reference images before committing them:
```bash
# One image
oxipng -o max path/to/image.png

# All images
oxipng -r -o max tests/*/ref
```
