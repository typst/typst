--- assert-fail eval ---
// Test failing assertions.
// Error: 2-16 assertion failed
#assert(1 == 2)

--- assert-fail-message eval ---
// Test failing assertions.
// Error: 2-51 assertion failed: two is smaller than one
#assert(2 < 1, message: "two is smaller than one")

--- assert-bad-type eval ---
// Test failing assertions.
// Error: 9-15 expected boolean, found string
#assert("true")

--- assert-eq-fail eval ---
// Test failing assertions.
// Error: 2-19 equality assertion failed: value 10 was not equal to 11
#assert.eq(10, 11)

--- assert-eq-fail-message eval ---
// Test failing assertions.
// Error: 2-55 equality assertion failed: 10 and 12 are not equal
#assert.eq(10, 12, message: "10 and 12 are not equal")

--- assert-ne-fail eval ---
// Test failing assertions.
// Error: 2-19 inequality assertion failed: value 11 was equal to 11
#assert.ne(11, 11)

--- assert-ne-fail-message eval ---
// Test failing assertions.
// Error: 2-57 inequality assertion failed: must be different from 11
#assert.ne(11, 11, message: "must be different from 11")

--- assert-success eval ---
// Test successful assertions.
#assert(5 > 3)
#assert.eq(15, 15)
#assert.ne(10, 12)

--- assert-message-escaped eval ---
// `assert` should escape escape characters.
// Error: 2-73 assertion failed: �[2JAre they � escaped?�
#assert(false, message: "\u{001B}[2JAre they \u{009C} escaped?\u{007F}")

--- assert-eq-message-escaped eval ---
// `assert.eq` should escape escape characters.
// Error: 2-75 equality assertion failed: �[2JAre they � escaped?�
#assert.eq(1, 2, message: "\u{001B}[2JAre they \u{009C} escaped?\u{007F}")

--- assert-ne-message-escaped eval ---
// `assert.ne` should escape escape characters.
// Error: 2-75 inequality assertion failed: �[2JAre they � escaped?�
#assert.ne(1, 1, message: "\u{001B}[2JAre they \u{009C} escaped?\u{007F}")
