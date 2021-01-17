// Ref: false

#let a = 2;
#let b = 4;

// Paren call.
[eq f(1), "f(1)"]
[eq type(1), "integer"]

// Unary operations.
[eq +1, 1]
[eq -1, 1-2]
[eq --1, 1]

// Binary operations.
[eq "a" + "b", "ab"]
[eq 1-4, 3*-1]
[eq a * b, 8]
[eq 12pt/.4, 30pt]

// Associativity.
[eq 1+2+3, 6]
[eq 1/2*3, 1.5]

// Precedence.
[eq 1+2*-3, -5]

// Parentheses.
[eq (a), 2]
[eq (2), 2]
[eq (1+2)*3, 9]

// Confusion with floating-point literal.
[eq 1e+2-1e-2, 99.99]

// Error: 1:3-1:3 expected expression
{-}

// Error: 1:8-1:8 expected expression
[eq {1+}, 1]

// Error: 1:8-1:8 expected expression
[eq {2*}, 2]
