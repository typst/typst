# Tests

Directory structure:
- `typ`: Input files.
- `ref`: Reference images which the output is compared with to determine whether
         a test passed or failed.
- `res`: Resource files used by tests.
- `png`: PNG files produced by tests.
- `pdf`: PDF files produced by tests.

The test files are split into three categories:
- `full`: Tests of full documents.
- `lang`: Tests for specific language features.
- `library`: Tests for specific library functions.

To keep things small, please optimize the reference images:
```bash
# One image
oxipng -o max tests/ref/image.png

# All images
oxipng -r -o max tests/ref/*
```
