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
#{
  // Error: 8-18 unclosed delimiter
  // Error: 8-18 maximum parsing depth exceeded
  eval(1024 * "(")
}
