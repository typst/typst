--- warn-show-set-last-in-block ---
#{
  // Warning: 1-14 show rule has no effect
  // Hint: 3-16 a show rule is only in effect until the end of the surrounding code block
  show "a": "b"
}

#{
  // Warning: 1-15 set rule has no effect
  // Hint: 3-17 a set rule is only in effect until the end of the surrounding code block
  set text(blue)
}
