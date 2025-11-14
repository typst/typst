// Test math syntax.

--- math-unicode paged ---
// Test Unicode math.
$ ∑_(i=0)^ℕ a ∘ b = \u{2211}_(i=0)^NN a compose b $

--- math-shorthands paged ---
// Test a few shorthands.
$ underline(f' : NN -> RR) \
  n |-> cases(
    [|1|] &"if" n >>> 10,
    2 * 3 &"if" n != 5,
    1 - 0 thick &...,
  ) $

--- math-shorthands-noncontinuable paged ---
// Test that shorthands are not continuable.
$ x >=(y) / z \
  x >= (y) / z $

--- math-common-symbols paged ---
// Test common symbols.
$ dot \ dots \ ast \ tilde \ star $

--- issue-2044-invalid-parsed-ident paged ---
// In this bug, the dot at the end was causing the right parenthesis to be
// parsed as an identifier instead of the closing right parenthesis.
$floor(phi.alt.)$
$floor(phi.alt. )$

--- issue-4828-math-number-multi-char paged ---
// Numbers should parse the same regardless of number of characters.
$1/2(x)$ vs. $1/10(x)$

--- math-unclosed paged ---
// Error: 1-2 unclosed delimiter
$a
