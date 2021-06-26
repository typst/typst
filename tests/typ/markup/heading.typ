// Test headings.

---
// Different number of hashtags.

// Valid levels.
# Level 1
### Level 2
###### Level 6

// Too many hashtags.
// Warning: 1-8 should not exceed depth 6
####### Level 7

---
// Heading vs. no heading.

// Parsed as headings if at start of the context.
/**/ # Level 1
{[## Level 2]}
#box[### Level 3]

// Not at the start of the context.
No # heading

// Escaped.
\# No heading

---
// While indented at least as much as the start, the heading continues.

# This
  is
    indented.

#  This
  is not.

// Code blocks continue heading.
# A {
    "B"
}
