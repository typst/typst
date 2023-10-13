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
