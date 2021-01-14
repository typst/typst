# Tests

- `typ`: Input files.
- `ref`: Reference images which the output is compared with to determine whether
         a test passed or failed. To keep things small, please run
         `oxipng -o max tests/ref/<img>` when creating or updating reference
         images (note that `<img>` can be `*` to optimize all images).
- `res`: Resource files used by tests.
- `png`: PNG files produced by tests.
- `pdf`: PDF files produced by tests.

The test files are split into three categories:
- `full`: Tests of full documents.
- `lang`: Tests for specific language features.
- `library`: Tests for specific library functions.
