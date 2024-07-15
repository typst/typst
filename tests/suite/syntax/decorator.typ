// Test decorators.

--- basic-decorators ---

// @allow()
// @allow("A")
// @allow("the")

// @allow("unnecessary-stars")
#h(0em)

#let _ = {
  // @allow("unnecessary-stars")
  h(0em)
}

#let _ = $
  // @allow("unnecessary-stars")
  h(#0em)
$

--- decorator-comments ---

// @allow("abc") // this is ok

// @allow("abc") /* this is ok */

// @allow("abc" /* this is ok */, "abc")

// @allow("abc" /*
this is ok
*/, "abc")

--- decorator-strings ---

// @allow("@some/thing-there123")

--- unknown-decorator ---
// Error: 2:5-2:13 invalid decorator name
// Hint: 2:5-2:13 must be 'allow'

// @whatever()

--- invalid-decorator-syntax ---
// Error: 2:11-2:12 the character '*' is not valid in a decorator

// @allow(*)

// Error: 2:11-2:12 the character '5' is not valid in a decorator

// @allow(5)

// Error: 2:5-2:19 expected identifier

// @555!**INVALID!

// Error: 2:10-2:13 expected opening paren

// @allow)")

// Error: 2:11-2:15 unclosed string
// Error: 2:15 expected closing paren

// @allow("abc

// Error: 2:18-2:21 expected end of decorator

// @allow("abc") abc

// Error: 2:17-2:22 expected comma
// Error: 2:24-2:27 expected end of decorator

// @allow("abc" "abc") abc

// Error: 2:17-2:22 expected comma

// @allow("abc" "abc", "abc")

// Error: 2:11-2:12 unexpected comma

// @allow(,  "abc", "abc", "abc")

--- invalid-decorator-strings ---

// Error: 2:11-2:16 invalid character ' ' in a decorator's string

// @allow("a b")

// Error: 2:11-2:19 invalid character '|' in a decorator's string

// @allow("aaaaa|")

// TODO: Why does this print / instead of \?
// Error: 2:11-2:19 invalid character '/' in a decorator's string

// @allow("aaaaa\")

--- allow-suppresses-warns-below ---

// @allow("unnecessary-stars")
#[**]

// @allow("unnecessary-stars")
#{
  {
    [**]
  }
}

/**/ // @allow("unnecessary-stars")
#[**]

// @allow("unnecessary-stars")
**

--- allow-suppresses-warn-with-tracepoint ---
#let f() = {
  text(font: "Unbeknownst")[]
}

#let g() = {
  f()
}

// @allow("unknown-font-families")
#g()

--- allow-suppresses-line-below-but-not-same-line ---
// Warning: 3-5 no text within stars
// Hint: 3-5 using multiple consecutive stars (e.g. **) has no additional effect
#[**] // @allow("unnecessary-stars")
#[**]

--- allow-before-parbreak-doesnt-suppress-warn ---
// Warning: 4:3-4:5 no text within stars
// Hint: 4:3-4:5 using multiple consecutive stars (e.g. **) has no additional effect

// @allow("unnecessary-stars")

#[**]

--- allow-before-empty-code-line-doesnt-suppress-warn ---
// Warning: 4:4-4:6 no text within stars
// Hint: 4:4-4:6 using multiple consecutive stars (e.g. **) has no additional effect
#{
  // @allow("unnecessary-stars")

  [**]
}

--- unattached-allow-doesnt-suppress-warn ---

// Warning: 1-3 no text within stars
// Hint: 1-3 using multiple consecutive stars (e.g. **) has no additional effect
**

// @allow("unnecessary-stars")
#h(0em)
// Warning: 3-5 no text within stars
// Hint: 3-5 using multiple consecutive stars (e.g. **) has no additional effect
#[**]

--- allow-doesnt-suppress-warn-in-nested-context ---
// Warning: 2:14-2:27 unknown font family: unbeknownst
#let f() = context {
  text(font: "Unbeknownst")[]
}

// @allow("unknown-font-families")
#f()

// @allow("unknown-font-families")
#context {
  text(font: "Unbeknownst")[]
}
