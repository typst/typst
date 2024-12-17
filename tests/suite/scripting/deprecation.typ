// Test deprecation warnings.

// `contains-deprecated` is a module defined
// for tests, which contains a single value:
// `deprecated`, which is deprecated.

--- use-deprecated ---
#import contains-deprecated: deprecated
// Warning: 2-12 this is deprecated
#deprecated

--- use-deprecated-wildcard ---
#import contains-deprecated: *
// Warning: 2-12 this is deprecated
#deprecated

--- use-deprecated-field ---
#import contains-deprecated
// Warning: 22-32 this is deprecated
#contains-deprecated.deprecated

--- no-deprecation-warning-emitted-for-binding ---
#import contains-deprecated
// Warning: 30-40 this is deprecated
#let d = contains-deprecated.deprecated
// No warning here.
#d
