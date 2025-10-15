--- assert-fail ---
// Test failing assertions.
// Error: 2-16 assertion failed
#assert(1 == 2)

--- assert-fail-message ---
// Test failing assertions.
// Error: 2-51 assertion failed: two is smaller than one
#assert(2 < 1, message: "two is smaller than one")

--- assert-bad-type ---
// Test failing assertions.
// Error: 9-15 expected boolean, found string
#assert("true")

--- assert-eq-fail ---
// Test failing assertions.
// Error: 2-19 equality assertion failed: value 10 was not equal to 11
#assert.eq(10, 11)

--- assert-eq-fail-message ---
// Test failing assertions.
// Error: 2-55 equality assertion failed: 10 and 12 are not equal
#assert.eq(10, 12, message: "10 and 12 are not equal")

--- assert-ne-fail ---
// Test failing assertions.
// Error: 2-19 distinctness assertion failed: value 11 was equal to 11
#assert.ne(11, 11)

--- assert-ne-fail-message ---
// Test failing assertions.
// Error: 2-57 distinctness assertion failed: must be different from 11
#assert.ne(11, 11, message: "must be different from 11")

--- assert-success ---
// Test successful assertions.
#assert(5 > 3)
#assert.eq(15, 15)
#assert.ne(10, 12)
