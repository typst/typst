// Test arrays.
// Ref: false

---
// Ref: true

// Empty.
{()}

// Not an array, just a parenthesized expression.
{(1)}

// One item and trailing comma.
{(-1,)}

// No trailing comma.
{(true, false)}

// Multiple lines and items and trailing comma.
{("1"
     , rgb("002")
     ,)}

---
// Test lvalue and rvalue access.
{
  let array = (1, 2)
  array(1) += 5 + array(0)
  test(array, (1, 8))
}

---
// Test rvalue out of bounds.
{
  let array = (1, 2, 3)
  // Error: 3-11 array index out of bounds (index: 5, len: 3)
  array(5)
}

---
// Test lvalue out of bounds.
{
  let array = (1, 2, 3)
  // Error: 3-12 array index out of bounds (index: -1, len: 3)
  array(-1) = 5
}

---
// Test non-collection indexing.

{
  let x = 10pt
  // Error: 3-4 expected collection, found length
  x() = 1
}

---
// Error: 3 expected closing paren
{(}

// Error: 2-3 expected expression, found closing paren
{)}

// Error: 4 expected comma
// Error: 4-6 expected expression, found end of block comment
{(1*/2)}

// Error: 6-8 expected expression, found invalid token
{(1, 1u 2)}

// Error: 3-4 expected expression, found comma
{(,1)}

// Missing expression makes named pair incomplete, making this an empty array.
// Error: 3-5 expected expression, found named pair
{(a:)}

// Named pair after this is already identified as an array.
// Error: 6-10 expected expression, found named pair
{(1, b: 2)}
