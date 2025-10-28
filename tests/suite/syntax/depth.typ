--- parser-depth-exceeded-balanced ---
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
  eval(s)
}

--- parser-depth-exceeded-unbalanced ---
// Error: 7-17 unclosed delimiter
// Error: 7-17 maximum parsing depth exceeded
#eval(1024 * "(")

--- parser-depth-exceeded-unbalanced-arrow ---
// https://issues.oss-fuzz.com/issues/42538221
// Error: 7-20 the character `#` is not valid in code
// Error: 7-20 unclosed delimiter
// Error: 7-20 unexpected arrow
// Error: 7-20 maximum parsing depth exceeded
#eval(512 * "#((=>")

--- parser-depth-exceeded-unop ---
// https://issues.oss-fuzz.com/issues/415163163
// Error: 7-17 maximum parsing depth exceeded
// Error: 7-17 expected expression
#eval(512 * "- ")
