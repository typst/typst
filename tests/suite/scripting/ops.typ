// Test binary expressions.

--- ops-add-content paged ---
// Test adding content.
#([*Hello* ] + [world!])

--- ops-unary-basic eval ---
// Test math operators.

// Test plus and minus.
#for v in (1, 3.14, 12.43d, 12pt, 45deg, 90%, 13% + 10pt, 6.3fr) {
  // Test plus.
  test(+v, v)

  // Test minus.
  test(-v, -1 * v)
  test(--v, v)

  // Test combination.
  test(-++ --v, -v)
}

#test(-(4 + 2), 6-12)

// Addition.
#test(2 + 4, 6)
#test("a" + "b", "ab")
#test("a" + if false { "b" }, "a")
#test("a" + if true { "b" }, "ab")
#test(13 * "a" + "bbbbbb", "aaaaaaaaaaaaabbbbbb")
#test((1, 2) + (3, 4), (1, 2, 3, 4))
#test((a: 1) + (b: 2, c: 3), (a: 1, b: 2, c: 3))

--- ops-add-too-large eval ---
// Error: 3-14 value is too large
#(int.max + 1)

--- ops-binary-basic eval ---
// Subtraction.
#test(1-4, 3*-1)
#test(4cm - 2cm, 2cm)
#test(1e+2-1e-2, 99.99)

// Multiplication.
#test(2 * 4, 8)

// Division.
#test(12pt/.4, 30pt)
#test(7 / 2, 3.5)

// Combination.
#test(3-4 * 5 < -10, true)
#test({ let x; x = 1 + 4*5 >= 21 and { x = "a"; x + "b" == "ab" }; x }, true)

// With block.
#test(if true {
  1
} + 2, 3)

// Mathematical identities.
#let nums = (
  1, 3.14,
  12.45d,
  12pt, 3em, 12pt + 3em,
  45deg,
  90%,
  13% + 10pt, 5% + 1em + 3pt,
  2.3fr,
)

#for v in nums {
  // Test plus and minus.
  test(v + v - v, v)
  test(v - v - v, -v)

  // Test plus/minus and multiplication.
  test(v - v, 0 * v)
  test(v + v, 2 * v)

  // Integer or decimal addition does not give a float.
  if type(v) not in (int, decimal) {
    test(v + v, 2.0 * v)
  }

  if type(v) not in (relative, decimal) and ("pt" not in repr(v) or "em" not in repr(v)) {
    test(v / v, 1.0)
  }
}

// Make sure length, ratio and relative length
// - can all be added to / subtracted from each other,
// - multiplied with integers and floats,
// - divided by integers and floats.
#let dims = (10pt, 1em, 10pt + 1em, 30%, 50% + 3cm, 40% + 2em + 1cm)
#for a in dims {
  for b in dims {
    test(type(a + b), type(a - b))
  }

  for b in (7, 3.14) {
    test(type(a * b), type(a))
    test(type(b * a), type(a))
    test(type(a / b), type(a))
  }
}

// Test division of different numeric types with zero components.
#for a in (0pt, 0em, 0%) {
  for b in (10pt, 10em, 10%) {
    test((2 * b) / b, 2)
    test((a + b * 2) / b, 2)
    test(b / (b * 2 + a), 0.5)
  }
}

--- ops-binary-decimal eval ---
// Addition.
#test(40.1d + 13.2d, 53.3d)
#test(12.34330d + 45.96670d, 58.31000d)
#test(451113.111111111111111111111d + 23.222222222222222222324d, 451136.333333333333333333435d)

// Subtraction.
#test(40.1d - 13.2d, 26.9d)
#test(12.34330d - 45.96670d, -33.62340d)
#test(1234.111111111111111111111d - 0.222222222222222222324d, 1233.888888888888888888787d)

// Multiplication.
#test(40.5d * 9.5d, 384.75d)
#test(-0.1234567890123456789012345678d * -2.0d, 0.2469135780246913578024691356d)

// Division.
#test(1.0d / 7.0d, 0.1428571428571428571428571429d)
#test(9999991.6666d / 3.0d, 3333330.5555333333333333333333d)
#test(3253452.4034029359598214312040d / -49293591.4039493929532d, -0.0660015290170614346071165643d)

--- ops-binary-decimal-int eval ---
// Operations between decimal and integer.
#test(2359.123456789123456789001234d + 2, 2361.123456789123456789001234d)
#test(2359.123456789123456789001234d - 2, 2357.123456789123456789001234d)
#test(2359.123456789123456789001234d * 2, 4718.246913578246913578002468d)
#test(2359.123456789123456789001234d / 2, 1179.561728394561728394500617d)

--- ops-binary-decimal-multiplication-division-imprecision eval ---
// Test digit truncation by multiplication and division.
#test(0.7777777777777777777777777777d / 1000, 0.0007777777777777777777777778d)
#test(0.7777777777777777777777777777d * 0.001d, 0.0007777777777777777777777778d)

--- ops-add-too-large-decimal eval ---
// Error: 3-37 value is too large
#(79228162514264337593543950335d + 1)

--- ops-subtract-too-large-decimal eval ---
// Error: 3-38 value is too large
#(-79228162514264337593543950335d - 1)

--- ops-multiply-inf-with-length paged ---
// Test that multiplying infinite numbers by certain units does not crash.
#(float("inf") * 1pt)
#(float("inf") * 1em)
#(float("inf") * (1pt + 1em))

--- ops-attempt-nan-length eval ---
// Test that trying to produce a NaN scalar (such as in lengths) does not crash.
#let infpt = float("inf") * 1pt
#test(infpt - infpt, 0pt)
#test(infpt + (-infpt), 0pt)
// TODO: this result is surprising
#test(infpt / float("inf"), 0pt)

--- ops-unary-bool eval ---
// Test boolean operators.

// Test not.
#test(not true, false)
#test(not false, true)

// And.
#test(false and false, false)
#test(false and true, false)
#test(true and false, false)
#test(true and true, true)

// Or.
#test(false or false, false)
#test(false or true, true)
#test(true or false, true)
#test(true or true, true)

// Short-circuiting.
#test(false and dont-care, false)
#test(true or dont-care, true)

--- ops-equality eval ---
// Test equality operators.

// Most things compare by value.
#test(1 == "hi", false)
#test(1 == 1.0, true)
#test(30% == 30% + 0cm, true)
#test(1in == 0% + 72pt, true)
#test(30% == 30% + 1cm, false)
#test("ab" == "a" + "b", true)
#test(() == (1,), false)
#test((1, 2, 3) == (1, 2.0) + (3,), true)
#test((:) == (a: 1), false)
#test((a: 2 - 1.0, b: 2) == (b: 2, a: 1), true)
#test("a" != "a", false)
#test(1.234d == 1.23400000000d, true)
#test(235 == 235.0d, true)

// Functions compare by identity.
#test(test == test, true)
#test((() => {}) == (() => {}), false)

// Content compares field by field.
#let t = [a]
#test(t == t, true)
#test([] == [], true)
#test([a] == [a], true)
#test(grid[a] == grid[a], true)
#test(grid[a] == grid[b], false)

--- ops-compare eval ---
// Test comparison operators.

#test(13 * 3 < 14 * 4, true)
#test(5 < 10, true)
#test(5 > 5, false)
#test(5 <= 5, true)
#test(5 <= 4, false)
#test(45deg < 1rad, true)
#test(10% < 20%, true)
#test(50% < 40% + 0pt, false)
#test(40% + 0pt < 50% + 0pt, true)
#test(1em < 2em, true)
#test((0, 1, 2, 4) < (0, 1, 2, 5), true)
#test((0, 1, 2, 4) < (0, 1, 2, 3), false)
#test((0, 1, 2, 3.3) > (0, 1, 2, 4), false)
#test((0, 1, 2) < (0, 1, 2, 3), true)
#test((0, 1, "b") > (0, 1, "a", 3), true)
#test((0, 1.1, 3) >= (0, 1.1, 3), true)
#test((0, 1, datetime(day: 1, month: 12, year: 2023)) <= (0, 1, datetime(day: 1, month: 12, year: 2023), 3), true)
#test(("a", 23, 40, "b") > ("a", 23, 40), true)
#test(() <= (), true)
#test(() >= (), true)
#test(() <= (1,), true)
#test((1,) <= (), false)
#test(123.0000000000000000000000001d > 123.0d, true)
#test(123.5d < 122.444d, false)
#test(459.9999999999999999999999999d < 460, true)
#test(128.50d > 460, false)

--- ops-in eval ---
// Test `in` operator.
#test("hi" in "worship", true)
#test("hi" in ("we", "hi", "bye"), true)
#test("Hey" in "abHeyCd", true)
#test("Hey" in "abheyCd", false)
#test(5 in range(10), true)
#test(12 in range(10), false)
#test("" in (), false)
#test("key" in (key: "value"), true)
#test("value" in (key: "value"), false)
#test("Hey" not in "abheyCd", true)
#test("a" not
/* fun comment? */ in "abc", false)
#test("sys" in std, true)
#test("system" in std, false)

--- ops-in-cannot-apply-to-elem eval ---
// If this will become possible, the contains function needs to be adjusted to
// consider feature gates.
// Error: 3-25 cannot apply 'in' to string and function
#("whatever" in document)

--- ops-not-trailing eval ---
// Error: 10 expected keyword `in`
#("a" not)

--- func-with eval ---
// Test `with` method.

// Apply positional arguments.
#let add(x, y) = x + y
#test(add.with(2)(3), 5)
#test(add.with(2, 3)(), 5)
#test(add.with(2).with(3)(), 5)
#test((add.with(2))(4), 6)
#test((add.with(2).with(3))(), 5)

// Make sure that named arguments are overridable.
#let inc(x, y: 1) = x + y
#test(inc(1), 2)

#let inc2 = inc.with(y: 2)
#test(inc2(2), 4)
#test(inc2(2, y: 4), 6)

// Apply arguments to an argument sink.
#let times(..sink) = {
  let res = sink.pos().product()
  if sink.named().at("negate", default: false) { res *= -1 }
  res
}
#test((times.with(2, negate: true).with(5))(), -10)
#test((times.with(2).with(5).with(negate: true))(), -10)
#test((times.with(2).with(5, negate: true))(), -10)
#test((times.with(2).with(negate: true))(5), -10)

--- ops-precedence-basic eval ---
// Multiplication binds stronger than addition.
#test(1+2*-3, -5)

// Subtraction binds stronger than comparison.
#test(3 == 5 - 2, true)

// Boolean operations bind stronger than '=='.
#test("a" == "a" and 2 < 3, true)
#test(not "b" == "b", false)

--- ops-precedence-boolean-ops eval ---
// Assignment binds stronger than boolean operations.
#let x = false
// Error: 3-8 cannot mutate a temporary value
#(not x = "a")

--- ops-precedence-unary eval ---
// Precedence doesn't matter for chained unary operators.
// Error: 3-12 cannot apply '-' to boolean
#(-not true)

--- ops-precedence-not-in eval ---
// Not in handles precedence.
#test(-1 not in (1, 2, 3), true)

--- ops-precedence-parentheses eval ---
// Parentheses override precedence.
#test((1), 1)
#test((1+2)*-3, -9)

// Error: 8-9 unclosed delimiter
#test({(1 + 1}, 2)

--- ops-associativity-left eval ---
// Math operators are left-associative.
#test(10 / 2 / 2 == (10 / 2) / 2, true)
#test(10 / 2 / 2 == 10 / (2 / 2), false)
#test(1 / 2 * 3, 1.5)

--- ops-associativity-right eval ---
// Assignment is right-associative.
#{
  let x = 1
  let y = 2
  x = y = "ok"
  test(x, none)
  test(y, "ok")
}

--- ops-unary-minus-missing-expr eval ---
// Error: 4 expected expression
#(-)

--- ops-unary-embedded-error eval ---
// Error: 2-3 unexpected minus
// Hint: 2-3 to use a unary operator here, wrap the entire expression in parentheses
#-30deg
// Error: 2-3 unexpected plus
// Hint: 2-3 to use a unary operator here, wrap the entire expression in parentheses
#+.5
// Error: 2-5 unexpected operator `not`
// Hint: 2-5 to use a unary operator here, wrap the entire expression in parentheses
#not
// Error: 23-26 unexpected operator `not`
// Hint: 23-26 to use a unary operator here, wrap the entire expression in parentheses
$harpoon(i, dotless: #not false)$

--- ops-unary-context-error eval ---
// Error: 10-11 unexpected minus
// Hint: 10-11 to use a unary operator here, wrap the entire expression in parentheses
#context -text.size
// Error: 10-13 unexpected operator `not`
// Hint: 10-13 to use a unary operator here, wrap the entire expression in parentheses
#context not par.justify

--- ops-add-missing-rhs eval ---
// Error: 10 expected expression
#test({1+}, 1)

--- ops-mul-missing-rhs eval ---
// Error: 10 expected expression
#test({2*}, 2)

--- ops-bad-token-rhs eval ---
// Error: 7-10 invalid number suffix: `p`
#(1 + 12p)
// Error: 7-8 the character `~` is not valid in code
#(1 / ~)

--- ops-bad-token-lhs eval ---
// Error: 3-4 the character `\` is not valid in code
// Error: 5-6 unexpected star
#(\ * 1)
// Error: 3-7 unclosed label
// Error: 8-11 unexpected operator `and`
#(<lbl and 1)

--- ops-unary-plus-on-content eval ---
// Error: 3-13 cannot apply unary '+' to content
#(+([] + []))

--- ops-unary-plus-on-string eval ---
// Error: 3-6 cannot apply '-' to string
#(-"")

--- ops-not-on-array eval ---
// Error: 3-9 cannot apply 'not' to array
#(not ())

--- ops-compare-relative-length-and-ratio eval ---
// Error: 3-19 cannot compare relative length and ratio
#(30% + 1pt <= 40%)

--- ops-compare-em-with-abs eval ---
// Error: 3-14 cannot compare 1em with 10pt
#(1em <= 10pt)

--- ops-compare-normal-float-with-nan eval ---
// Error: 3-22 cannot compare 2.2 with float.nan
#(2.2 <= float("nan"))

--- ops-compare-int-and-str eval ---
// Error: 3-26 cannot compare integer and string
#((0, 1, 3) > (0, 1, "a"))

--- ops-compare-array-nested-failure eval ---
// Error: 3-42 cannot compare 3.5 with float.nan
#((0, "a", 3.5) <= (0, "a", float("nan")))

--- ops-divide-by-zero-float eval ---
// Error: 3-12 cannot divide by zero
#(1.2 / 0.0)

--- ops-divide-by-zero-int eval ---
// Error: 3-8 cannot divide by zero
#(1 / 0)

--- ops-divide-by-zero-angle eval ---
// Error: 3-15 cannot divide by zero
#(15deg / 0deg)

--- ops-binary-arithmetic-error-message eval ---
// Special messages for +, -, * and /.
// Error: 3-10 cannot add integer and string
#(1 + "2", 40% - 1)

--- add-assign-int-and-str eval ---
// Error: 15-23 cannot add integer and string
#{ let x = 1; x += "2" }

--- ops-divide-ratio-by-length eval ---
// Error: 4-13 cannot divide ratio by length
#( 10% / 5pt )

--- ops-divide-em-by-abs eval ---
// Error: 3-12 cannot divide these two lengths
#(1em / 5pt)

--- ops-divide-relative-length-by-ratio eval ---
// Error: 3-19 cannot divide relative length by ratio
#((10% + 1pt) / 5%)

--- ops-divide-relative-lengths eval ---
// Error: 3-28 cannot divide these two relative lengths
#((10% + 1pt) / (20% + 1pt))

--- ops-subtract-int-from-ratio eval ---
// Error: 13-20 cannot subtract integer from ratio
#((1234567, 40% - 1))

--- ops-multiply-int-with-bool eval ---
// Error: 3-11 cannot multiply integer with boolean
#(2 * true)

--- ops-divide-int-by-length eval ---
// Error: 3-11 cannot divide integer by length
#(3 / 12pt)

--- multiply-negative-int-with-str eval ---
// Error: 3-10 number must be at least zero
#(-1 * "")

--- ops-assign eval ---
// Test assignment operators.

#let x = 0
#(x = 10)       #test(x, 10)
#(x -= 5)       #test(x, 5)
#(x += 1)       #test(x, 6)
#(x *= x)       #test(x, 36)
#(x /= 2.0)     #test(x, 18.0)
#(x = "some")   #test(x, "some")
#(x += "thing") #test(x, "something")

--- ops-assign-unknown-var-lhs eval ---
#{
  // Error: 3-6 unknown variable `a-1`
  // Hint: 3-6 if you meant to use subtraction, try adding spaces around the minus sign: `a - 1`
  a-1 = 2
}

--- ops-assign-unknown-var-rhs eval ---
#{
  let a = 2
  a = 1-a
  a = a -1

  // Error: 7-10 unknown variable `a-1`
  // Hint: 7-10 if you meant to use subtraction, try adding spaces around the minus sign: `a - 1`
  a = a-1
}

--- ops-assign-unknown-parenthesized-variable eval ---
// Error: 4-5 unknown variable `x`
#((x) = "")

--- ops-assign-destructuring-unknown-variable eval ---
// Error: 4-5 unknown variable `x`
#((x,) = (1,))

--- ops-assign-to-temporary eval ---
// Error: 3-8 cannot mutate a temporary value
#(1 + 2 += 3)

--- ops-assign-to-temporary-method eval ---
#let numbers = (3, 2, 1)
// Error: 3-19 cannot mutate a temporary value
#(numbers.sorted() = (1, 2, 3))

--- ops-assign-to-invalid-unary-op eval ---
#let x = "Hey"
// Error: 3-8 cannot apply 'not' to string
#(not x = "a")

--- ops-assign-to-invalid-binary-op eval ---
// Error: 7-8 unknown variable `x`
#(1 + x += 3)

--- ops-assign-unknown-variable eval ---
// Error: 3-4 unknown variable `z`
#(z = 1)

--- ops-assign-to-std-constant eval ---
// Error: 3-7 cannot mutate a constant: rect
#(rect = "hi")

--- ops-assign-to-shadowed-std-constant eval ---
// Works if we define rect beforehand
// (since then it doesn't resolve to the standard library version anymore).
#let rect = ""
#(rect = "hi")

--- ops-assign-shadow-eval-order eval ---
// Test shadowing a variable while assigning to it and calling a method on it.
#{
  let var = "a"
  var += var.at(0, default: let var = "b")
  test(var, "ba")
}
