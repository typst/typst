// Number of hashtags.
//
// warning: 5:1-5:8 section depth should not exceed 6

# One
### Three
###### Six
####### Seven

---
// Is a heading.

/**/ # Heading
{[## Heading]}
[box][### Heading]

---
// Is no heading.
//
// error: 4:1-4:6 unexpected invalid token

\# No heading

Text with # hashtag

#nope

---
// Heading continues over linebreak.

# This {
    "works"
}

# [box][
    This
] too

# This
does not
