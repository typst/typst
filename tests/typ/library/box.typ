// Test the box function.

---
#page("a7", flip: true)

// Box with fixed width, should have text height.
#box(width: 2cm, color: #9650D6)[A]

Sometimes there is no box.

// Box with fixed height, should span line.
#box(height: 2cm, width: 100%, color: #734CED)[B]

// Empty box with fixed width and height.
#box(width: 6cm, height: 12pt, color: #CB4CED)

// Not visible, but creates a gap between the boxes above and below.
#box(width: 2in, color: #ff0000)

// These are in a row!
#box(width: 1in, height: 10pt, color: #D6CD67)
#box(width: 1in, height: 10pt, color: #EDD466)
#box(width: 1in, height: 10pt, color: #E3BE62)
