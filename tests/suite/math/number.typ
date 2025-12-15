// Test numbers in math.

--- math-number-parsed paged ---
// Test the number is correctly rendered, separately from number-like text.
#show math.number: set text(red)

#let a0 = math.attach(math.alpha, b: [$0$])
#let b0 = math.attach(math.beta, b: [0])
$
  "num"  & 2, 2.1, n_2, n/2, mat(1, 2), a0 \
  "text" & "2"#footnote[footnote], "2.1", n_"2", n/#[2], mat("1", #[2]), b0
$

--- math-number-coerced paged ---
// Test the number-like text is rendered like a number.
// Notably, the comma "," won't be followed by a space if it's used as a
// digit grouping delimiter in the number() call.
#show math.number: set text(red)
$ number("2"), number("2.1"), number("1.0.1") \
  number("1,000.01") "vs" 1,000.01 \
  number("1'000.01") "vs" 1'000.01 $
