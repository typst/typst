// Test newline continuations.

---
#{
  "hello"
    .clusters()
  if false {

  }
  else {
    ("1", "2")
  }
}

---
#"hello"
  .codepoints()

#if false {

}
else {
  ("1", "2")
}

---
#{
  "hi 1"

    .clusters()
}

---
#{
  "hi 2"// comment
    .clusters()
}

---
#{
  "hi 3"/* comment */
    .clusters()
}

---
#{
  "hi 4"
  // comment
    .clusters()
}

---
#{
  "hi 5"
  /*comment*/.clusters()
}

---
#{
  "hi 6"
  // comment


  /* comment */
    .clusters()
}

---
#let foo(x) = {
  if x < 0 { "negative" }
  // comment
  else { "non-negative" }
}
#foo(1)
