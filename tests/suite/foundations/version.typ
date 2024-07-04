// Test versions.

--- version-constructor ---
// Test version constructor.

// Empty.
#test(array(version()), ())

// Plain.
#test(version(1, 2).major, 1)

// Single Array argument.
#test(version((1, 2)).minor, 2)

// Mixed arguments.
#test(version(1, (2, 3), 4, (5, 6), 7).at(5), 6)

--- version-equality ---
// Test equality of different-length versions
#test(version(), version(0))
#test(version(0), version(0, 0))
#test(version(1, 2), version(1, 2, 0, 0, 0, 0))

--- version-at ---
// Test `version.at`.

// Non-negative index in bounds
#test(version(1, 2).at(1), 2)

// Non-negative index out of bounds
#test(version(1, 2).at(4), 0)

// Negative index in bounds
#test(version(1, 2).at(-2), 1)

// Error: 2-22 component index out of bounds (index: -3, len: 2)
#version(1, 2).at(-3)

--- version-fields ---
// Test version fields.
#test(version(1, 2, 3).major, 1)
#test(version(1, 2, 3).minor, 2)
#test(version(1, 2, 3).patch, 3)

--- version-type ---
// Test the type of `sys.version`
#test(type(sys.version), version)
