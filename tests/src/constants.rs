//! Paths and other constants used for the test runner.

/// The directory where the test suite is located.
pub const SUITE_PATH: &str = "tests/suite";

/// The directory where the full test results are stored.
pub const STORE_PATH: &str = "tests/store";

/// The directory where the reference images are stored.
pub const REF_PATH: &str = "tests/ref";

/// The maximum size of reference images that aren't marked as `// LARGE`.
pub const REF_LIMIT: usize = 20 * 1024;
