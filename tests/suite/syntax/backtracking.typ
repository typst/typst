// Ensure that parser backtracking doesn't lead to exponential time consumption.
// If this regresses, the test suite will not terminate, which is a bit
// unfortunate compared to a good error, but at least we know something is up.
//

--- parser-backtracking-param-default-value render ---
#{
  let s = "(x: 1) => x"
  let pat = "(x: {}) => 1 + x()"
  for _ in range(50) {
    s = pat.replace("{}", s)
  }
  test(eval(s)(), 51)
}

--- parser-backtracking-destructuring-assignment render ---
#{
  let s = "(x) = 1"
  let pat = "(x: {_}) = 1"
  for _ in range(100) {
    s = pat.replace("_", s)
  }
  // Error: 8-9 cannot destructure integer
  eval(s)
}

--- parser-backtracking-destructuring-whitespace render ---
// Test whitespace after memoized part.
#( (x: () => 1 ) => 1 )
//     -------
//     This is memoized and we want to ensure that whitespace after this
//     is handled correctly.
