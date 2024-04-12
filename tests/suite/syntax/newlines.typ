// Test newline continuations.

--- newline-continuation-code ---
#{
  "hello"
    .clusters()
  if false {

  }
  else {
    ("1", "2")
  }
}

--- newline-continuation-markup ---
#"hello"
  .codepoints()

#if false {

}
else {
  ("1", "2")
}

--- newline-continuation-method-blank ---
#test({
  "hi 1"

    .clusters()
}, ("h", "i", " ", "1"))

--- newline-continuation-method-line-comment-after ---
#test({
  "hi 2"// comment
    .clusters()
}, ("h", "i", " ", "2"))

--- newline-continuation-method-block-comment-after ---
#test({
  "hi 3"/* comment */
    .clusters()
}, ("h", "i", " ", "3"))

--- newline-continuation-method-line-comment-between ---
#test({
  "hi 4"
  // comment
    .clusters()
}, ("h", "i", " ", "4"))

--- newline-continuation-method-block-comment-between ---
#test({
  "hi 5"
  /*comment*/.clusters()
}, ("h", "i", " ", "5"))

--- newline-continuation-method-comments-and-blanks ---
#test({
  "hi 6"
  // comment


  /* comment */
    .clusters()
}, ("h", "i", " ", "6"))

--- newline-continuation-if-else-comment ---
#test({
  let foo(x) = {
    if x < 0 { "negative" }
    // comment
    else { "non-negative" }
  }

  foo(1)
}, "non-negative")
