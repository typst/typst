--- parser-depth-exceeded-balanced eval ---
#{
  let s = "()"
  let pat = "({})"
  for _ in range(255) {
    s = pat.replace("{}", s)
  }
  // Ok
  eval(s)

  s = pat.replace("{}", s)
  // Error: 8-9 maximum parsing depth exceeded
  // Hint: 8-9 at index range: 256 to 258
  eval(s)
}

--- parser-depth-exceeded-unbalanced eval ---
// Error: 7-17 unclosed delimiter
// Hint: 7-17 at index range: 0 to 1
// Error: 7-17 maximum parsing depth exceeded
// Hint: 7-17 at index range: 256 to 1024
#eval(1024 * "(")

--- parser-depth-exceeded-unbalanced-arrow eval ---
// https://issues.oss-fuzz.com/issues/42538221
// Error: 7-20 the character `#` is not valid in code
// Hint: 7-20 you are already in code mode
// Hint: 7-20 at index range: 0 to 1
// Hint: 7-20 try removing the `#`
// Error: 7-20 unclosed delimiter
// Hint: 7-20 at index range: 1 to 2
// Error: 7-20 unexpected arrow
// Hint: 7-20 at index range: 3 to 5
// Error: 7-20 maximum parsing depth exceeded
// Hint: 7-20 at index range: 641 to 2560
#eval(512 * "#((=>")

--- parser-depth-exceeded-unop eval ---
// https://issues.oss-fuzz.com/issues/415163163
// Error: 7-17 maximum parsing depth exceeded
// Hint: 7-17 at index range: 512 to 513
// Error: 7-17 expected expression
// Hint: 7-17 at index: 1023
#eval(512 * "- ")
