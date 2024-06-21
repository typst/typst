// Test decorators.

--- decorators ---

/! allow()
/! allow("A")
/! allow(5)
/! allow("the")

/! allow("unnecessary-stars")
#[*a*]

#{
  /! allow("unnecessary-stars")
  [*a*]
}

$
  /! allow("unnecessary-stars")
  #[*a*]
$

--- unknown-decorator ---
/! whatever()

--- invalid-decorator ---
// Error: 1-13 the character * is not valid in a decorator
/! invalid(*)

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
b
// Warning: 3-5 no text within stars
// Hint: 3-5 using multiple consecutive stars (e.g. **) has no additional effect
#[**]
