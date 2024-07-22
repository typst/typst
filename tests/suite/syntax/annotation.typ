// Test annotations.

--- basic-annotations ---

// @allow
// @allow identifier
// @allow "quoted"
// @allow("parenthesized")

// @allow unnecessary-stars
#h(0em)

#let _ = {
  // @allow unnecessary-stars
  h(0em)
}

#let _ = $
  // @allow unnecessary-stars
  h(#0em)
$

--- annotation-comments ---
// Error: 2:17-2:18 the character '/' is not valid in an annotation
// Error: 2:18-2:19 the character '/' is not valid in an annotation

// @allow "abc" // comment

// Error: 2:17-2:18 the character '/' is not valid in an annotation
// Error: 2:18-2:19 the character '*' is not valid in an annotation
// Error: 2:28-2:29 the character '*' is not valid in an annotation
// Error: 2:29-2:30 the character '/' is not valid in an annotation

// @allow "abc" /* comment */

// Error: 2:17-2:18 the character '/' is not valid in an annotation
// Error: 2:18-2:19 the character '*' is not valid in an annotation
// Error: 2:28-2:29 the character '*' is not valid in an annotation
// Error: 2:29-2:30 the character '/' is not valid in an annotation

// @allow "abc" /* comment */ "abc"

--- annotation-strings ---

// @allow("@some/thing-there123")

--- unknown-annotation ---
// Error: 2:5-2:13 invalid annotation name
// Hint: 2:5-2:13 must be 'allow'

// @whatever A

--- invalid-annotation-syntax ---
// Error: 2:11-2:12 the character '*' is not valid in an annotation

// @allow *

// Error: 2:11-2:12 the character '5' is not valid in an annotation

// @allow 5

// Error: 2:5-2:19 expected identifier

// @555!**INVALID!

// Error: 2:10-2:11 the character ')' is not valid in an annotation
// Error: 2:11-2:13 unclosed string

// @allow)")

// Error: 2:11-2:15 unclosed string
// Error: 2:15 expected closing paren

// @allow("abc

// Error: 2:16-2:19 expected end of annotation

// @allow(abc) abc

// Error: 2:18-2:19 the character ',' is not valid in an annotation

// @allow(abc abc, "abc")

--- invalid-annotation-strings ---

// Error: 2:11-2:16 invalid character ' ' in an annotation's string

// @allow "a b"

// Error: 2:11-2:19 invalid character '|' in an annotation's string

// @allow "aaaaa|"

// TODO: Why does this print / instead of \?
// Error: 2:11-2:19 invalid character '/' in an annotation's string

// @allow "aaaaa\"

--- invalid-annotation-in-annotation ---
// Error: 2:17-2:18 the character '/' is not valid in an annotation
// Error: 2:18-2:19 the character '/' is not valid in an annotation
// Error: 2:20-2:21 the character '@' is not valid in an annotation
// Error: 2:26-2:27 the character '(' is not valid in an annotation
// Error: 2:32-2:33 the character ')' is not valid in an annotation

// @allow "aaa" // @allow("bbb")

--- allow-suppresses-warns-below ---

// @allow unnecessary-stars
#[**]

// @allow unnecessary-stars
#{
  {
    [**]
  }
}

/**/ // @allow unnecessary-stars
#[**]

// @allow unnecessary-stars
**

--- allow-suppresses-warn-with-tracepoint ---
#let f() = {
  text(font: "Unbeknownst")[]
}

#let g() = {
  f()
}

// @allow unknown-font-families
#g()

--- allow-suppresses-line-below-but-not-same-line ---
// Warning: 3-5 no text within stars
// Hint: 3-5 using multiple consecutive stars (e.g. **) has no additional effect
#[**] // @allow unnecessary-stars
#[**]

--- allow-before-parbreak-doesnt-suppress-warn ---
// Warning: 4:3-4:5 no text within stars
// Hint: 4:3-4:5 using multiple consecutive stars (e.g. **) has no additional effect

// @allow unnecessary-stars

#[**]

--- allow-before-empty-code-line-doesnt-suppress-warn ---
// Warning: 4:4-4:6 no text within stars
// Hint: 4:4-4:6 using multiple consecutive stars (e.g. **) has no additional effect
#{
  // @allow unnecessary-stars

  [**]
}

--- unattached-allow-doesnt-suppress-warn ---

// Warning: 1-3 no text within stars
// Hint: 1-3 using multiple consecutive stars (e.g. **) has no additional effect
**

// @allow unnecessary-stars
#h(0em)
// Warning: 3-5 no text within stars
// Hint: 3-5 using multiple consecutive stars (e.g. **) has no additional effect
#[**]

--- allow-doesnt-suppress-warn-in-nested-context ---
// Warning: 2:14-2:27 unknown font family: unbeknownst
#let f() = context {
  text(font: "Unbeknownst")[]
}

// @allow unknown-font-families
#f()

// @allow unknown-font-families
#context {
  text(font: "Unbeknownst")[]
}
