// Test binary expressions.
// Ref: false

---
// Test adding content.
// Ref: true
{[*Hello* ] + [world!]}

---
// Test math operators.

// Test plus and minus.
#for v in (1, 3.14, 12pt, 45deg, 90%, 13% + 10pt, 6.3fr) {
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
#test(13 * "a" + "bbbbbb", "aaaaaaaaaaaaabbbbbb")
#test((1, 2) + (3, 4), (1, 2, 3, 4))
#test((a: 1) + (b: 2, c: 3), (a: 1, b: 2, c: 3))

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

// Mathematical identities.
#let nums = (1, 3.14, 12pt, 45deg, 90%, 13% + 10pt)
#for v in nums {
  // Test plus and minus.
  test(v + v - v, v)
  test(v - v - v, -v)

  // Test plus/minus and multiplication.
  test(v - v, 0 * v)
  test(v + v, 2 * v)

  // Integer addition does not give a float.
  if type(v) != "integer" {
    test(v + v, 2.0 * v)
  }

  // Linears cannot be divided by themselves.
  if type(v) != "relative length" {
    test(v / v, 1.0)
    test(v / v == 1, true)
  }
}

// Make sure length, relative and linear
// - can all be added to / subtracted from each other,
// - multiplied with integers and floats,
// - divided by floats.
#let dims = (10pt, 30%, 50% + 3cm)
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

---
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

---
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

// Functions compare by identity.
#test(test == test, true)
#test((() => {}) == (() => {}), false)

// Content compares by hash equality.
#let t = [a]
#test(t == t, true)
#test([] == [], true)
#test([a] == [a], true)
#test([[a]] == [a], true)
#test([] == [a], false)
#test(box[] == box[], true)
#test(box[a] == box[], false)

---
// Test comparison operators.

#test(13 * 3 < 14 * 4, true)
#test(5 < 10, true)
#test(5 > 5, false)
#test(5 <= 5, true)
#test(5 <= 4, false)
#test(45deg < 1rad, true)
#test(10% < 20%, true)
#test(50% < 40% + 0pt, false)

---
// Test assignment operators.

#let x = 0
{ x = 10 }       #test(x, 10)
{ x -= 5 }       #test(x, 5)
{ x += 1 }       #test(x, 6)
{ x *= x }       #test(x, 36)
{ x /= 2.0 }     #test(x, 18.0)
{ x = "some" }   #test(x, "some")
{ x += "thing" } #test(x, "something")

---
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

---
// Error: 9 expected keyword `in`
{"a" not}

---
// Test `with` operator.

// Apply positional arguments.
#let add(x, y) = x + y
#test((add with (2))(4), 6)

// Let .. with .. syntax.
#let f = add
#let f with (2)
#test(f(4), 6)

// Make sure that named arguments are overridable.
#let inc(x, y: 1) = x + y
#test(inc(1), 2)

#let inc with (y: 2)
#test(inc(2), 4)
#test(inc(2, y: 4), 6)
