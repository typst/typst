// Test decorators.

--- basic-decorators ---

/! allow()
/! allow("A")
/! allow("the")

/! allow("unnecessary-stars")
#h(0em)

#{
  /! allow("unnecessary-stars")
  h(0em)
}

#let _ = $
  /! allow("unnecessary-stars")
  h(#0em)
$

--- decorator-comments ---

/! allow("abc") // this is ok

/! allow("abc") /* this is ok */

/! allow("abc" /* this is ok */, "abc")

/! allow("abc" /*
this is ok
*/, "abc")

--- decorator-strings ---

/! allow("@some/thing-there123")

--- unknown-decorator ---
// Error: 4-12 invalid decorator name
// Hint: 4-12 must be 'allow'
/! whatever()

--- invalid-decorator-syntax ---
// Error: 10-11 the character '*' is not valid in a decorator
/! allow(*)

// Error: 10-11 the character '5' is not valid in a decorator
/! allow(5)

// Error: 4-18 expected identifier
/! 555!**INVALID!

// Error: 9-12 expected opening paren
/! allow)")

// Error: 10-14 unclosed string
// Error: 14 expected closing paren
/! allow("abc

// Error: 17-20 expected end of decorator
/! allow("abc") abc

// Error: 16-21 expected comma
// Error: 23-26 expected end of decorator
/! allow("abc" "abc") abc

// Error: 16-21 expected comma
/! allow("abc" "abc", "abc")

// Error: 10-11 unexpected comma
/! allow(,  "abc", "abc", "abc")

--- invalid-decorator-strings ---

// Error: 10-15 invalid character ' ' in a decorator's string
/! allow("a b")

// Error: 10-18 invalid character '|' in a decorator's string
/! allow("aaaaa|")

// TODO: Why does this print / instead of \?
// Error: 10-18 invalid character '/' in a decorator's string
/! allow("aaaaa\")

--- allow-suppresses-warns ---

/! allow("unnecessary-stars")
#[**]

/! allow("unnecessary-stars")
#{
  {
    [**]
  }
}

/**/ /! allow("unnecessary-stars")
#[**]

/! allow("unnecessary-stars")
**

--- allow-before-parbreak-doesnt-suppress-warn ---
// Warning: 3:3-3:5 no text within stars
// Hint: 3:3-3:5 using multiple consecutive stars (e.g. **) has no additional effect
/! allow("unnecessary-stars")

#[**]

--- allow-before-empty-code-line-doesnt-suppress-warn ---
// Warning: 4:4-4:6 no text within stars
// Hint: 4:4-4:6 using multiple consecutive stars (e.g. **) has no additional effect
#{
  /! allow("unnecessary-stars")

  [**]
}

--- unattached-allow-doesnt-suppress-warn ---

// Warning: 1-3 no text within stars
// Hint: 1-3 using multiple consecutive stars (e.g. **) has no additional effect
**

/! allow("unnecessary-stars")
#h(0em)
// Warning: 3-5 no text within stars
// Hint: 3-5 using multiple consecutive stars (e.g. **) has no additional effect
#[**]

--- allow-doesnt-suppress-warn-in-nested-context ---
// Warning: 2:14-2:27 unknown font family: unbeknownst
#let f() = context {
  text(font: "Unbeknownst")[]
}

/! allow("unknown-font-families")
#f()

/! allow("unknown-font-families")
#context {
  text(font: "Unbeknownst")[]
}
