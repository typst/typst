# Tests

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

To keep things small, please optimize the reference images:
```bash
# One image
oxipng -o max path/to/image.png

# All images
oxipng -r -o max tests/*/ref
```
