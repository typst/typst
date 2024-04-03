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
// Ref: false
#test({
  "hi 1"

    .clusters()
}, ("h", "i", " ", "1"))

---
// Ref: false
#test({
  "hi 2"// comment
    .clusters()
}, ("h", "i", " ", "2"))

---
// Ref: false
#test({
  "hi 3"/* comment */
    .clusters()
}, ("h", "i", " ", "3"))

---
// Ref: false
#test({
  "hi 4"
  // comment
    .clusters()
}, ("h", "i", " ", "4"))

---
// Ref: false
#test({
  "hi 5"
  /*comment*/.clusters()
}, ("h", "i", " ", "5"))

---
// Ref: false
#test({
  "hi 6"
  // comment


  /* comment */
    .clusters()
}, ("h", "i", " ", "6"))

---
// Ref: false
#test({
  let foo(x) = {
    if x < 0 { "negative" }
    // comment
    else { "non-negative" }
  }

  foo(1)
}, "non-negative")
