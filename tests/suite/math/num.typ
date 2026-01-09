// Test numbers in math.

--- math-num-parsed paged ---
// Test that number is correctly rendered, separately from number-like text.
#set page(width: auto)
#show math.num: set text(red)

#let a0 = math.attach(math.alpha, b: $0$)
#let b0 = math.attach(math.beta, b: [0])
$
  "num"  & 2, 2.1, n_2, n/2, mat(1, 2), a0 \
  "text" & "2"#footnote[footnote], "2.1", n_"2", n/#[2], mat("1", #[2]), b0
$

--- math-num-coerced paged ---
// Test that number-like text is rendered like a number.
// Notably, the comma "," won't be followed by a space if it's used as a
// digit grouping delimiter in the num() call.
#set page(width: auto)
#show math.num: set text(red)
$ num("2"), num("2.1"), num("1.0.1") \
  num("1,000.01") "vs" 1,000.01 \
  num("1'000.01") "vs" 1'000.01 $

--- math-num-func-check paged ---
// Test that number's constructor is math.num and we can extract
// its contained string from the .text field.
#let inspect(equation) = {
  assert(equation.func() == math.equation)
  let content = equation.at("body")
  let isint = content.func() == math.num and regex("^\d+$") in content.text;
  let istext = content.func() == text
  return (isint, istext)
}

#test(inspect($1$), (true, false))
#test(inspect($#[1]$), (false, true))  // Note #[..] creates a content block
