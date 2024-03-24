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
