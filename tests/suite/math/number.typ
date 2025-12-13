// Test numbers in math.

--- math-number-parsed paged ---
// Test the number is correctly identified from number-like text.
#show math.number: set text(red)

#let a0 = math.attach(math.alpha, b: [$0$])
#let b0 = math.attach(math.beta, b: [0])
$
  "num"  & 2, 2.1, n_2, n/2, mat(1, 2), a0 \
  "text" & "2"#footnote[footnote], "2.1", n_"2", n/"2", ("1" "2"), b0
$
