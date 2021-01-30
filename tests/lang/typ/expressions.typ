// Ref: false

#let a = 2
#let b = 4

// Error: 1:14-1:17 cannot apply '+' to string
#let error = +""

// Paren call.
#[test f(1), "f(1)"]
#[test type(1), "integer"]

// Unary operations.
#[test +1, 1]
#[test -1, 1-2]
#[test --1, 1]

// Math operations.
#[test "a" + "b", "ab"]
#[test 1-4, 3*-1]
#[test a * b, 8]
#[test 12pt/.4, 30pt]
#[test 1e+2-1e-2, 99.99]

// Associativity.
#[test 1+2+3, 6]
#[test 1/2*3, 1.5]

// Precedence.
#[test 1+2*-3, -5]

// Short-circuiting logical operators.
#[test not "a" == "b", true]
#[test not 7 < 4 and 10 == 10, true]
#[test 3 < 2 or 4 < 5, true]
#[test false and false or true, true]

// Right-hand side not even evaluated.
#[test false and dont-care, false]
#[test true or dont-care, true]

// Equality and inequality.
#[test "ab" == "a" + "b", true]
#[test [*Hi*] == [*Hi*], true]
#[test "a" != "a", false]
#[test [*] != [_], true]
#[test (1, 2, 3) == (1, 2) + (3,), true]
#[test () == (1,), false]
#[test (a: 1, b: 2) == (b: 2, a: 1), true]
#[test (:) == (a: 1), false]
#[test 1 == "hi", false]
#[test 1 == 1.0, true]
#[test 30% == 30% + 0cm, true]
#[test 1in == 0% + 72pt, true]
#[test 30% == 30% + 1cm, false]

// Comparisons.
#[test 13 * 3 < 14 * 4, true]
#[test 5 < 10, true]
#[test 5 > 5, false]
#[test 5 <= 5, true]
#[test 5 <= 4, false]
#[test 45deg < 1rad, true]

// Assignment.
#let x = "some"
#let y = "some"
#[test (x = y = "") == none and x == none and y == "", true]

// Modify-assign operators.
#let x = 0
{ x = 10 }       #[test x, 10]
{ x -= 5 }       #[test x, 5]
{ x += 1 }       #[test x, 6]
{ x *= x }       #[test x, 36]
{ x /= 2.0 }     #[test x, 18.0]
{ x = "some" }   #[test x, "some"]
{ x += "thing" } #[test x, "something"]

// Error: 1:3-1:4 unknown variable
{ z = 1 }

// Error: 1:3-1:6 cannot assign to this expression
{ (x) = "" }

// Error: 1:3-1:8 cannot assign to this expression
{ 1 + 2 = 3}

// Error: 1:3-1:6 cannot assign to a constant
{ box = "hi" }

// Works if we define box before (since then it doesn't resolve to the standard
// library version anymore).
#let box = ""; { box = "hi" }

// Parentheses.
#[test (a), 2]
#[test (2), 2]
#[test (1+2)*3, 9]

// Error: 1:3-1:3 expected expression
{-}

// Error: 1:11-1:11 expected expression
#[test {1+}, 1]

// Error: 1:11-1:11 expected expression
#[test {2*}, 2]

// Error: 1:8-1:17 cannot apply '-' to boolean
#[test -not true, error]

// Error: 1:2-1:8 cannot apply 'not' to array
{not ()}

// Error: 1:3-1:10 cannot apply '+' to integer and string
{(1 + "2")}

// Error: 1:2-1:12 cannot apply '<=' to relative and relative
{30% <= 40%}
