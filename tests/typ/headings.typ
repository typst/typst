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
// error: 8:1-8:6 unexpected invalid token

\# No heading

Text with # hashtag

Nr#1

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
