#let a = 2;
#let b = 4;

// Unary operations.
{+1}
{-1}
{--1}

// Binary operations.
{"a"+"b"}
{1-2}
{a * b}
{12pt/.4}

// Associativity.
{1+2+3}
{1/2*3}

// Precedence.
{1+2*-3}

// Parentheses.
{(a)}
{(2)}
{(1+2)*3}

// Confusion with floating-point literal.
{1e+2-1e-2}

// Error: 1:3-1:3 expected expression
{-}

// Error: 1:4-1:4 expected expression
{1+}

// Error: 1:4-1:4 expected expression
{2*}
