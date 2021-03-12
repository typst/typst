// Test shapes.

---
#page("a8", flip: true)

// Box with fixed width, should have text height.
#box(width: 2cm, color: #9650D6)[Legal]

Sometimes there is no box.

// Box with fixed height, should span line.
#box(height: 1cm, width: 100%, color: #734CED)[B]

// Empty box with fixed width and height.
#box(width: 6cm, height: 12pt, color: #CB4CED)

// Not visible, but creates a gap between the boxes above and below.
#box(width: 2in, color: #ff0000)

// These are in a row!
#box(width: 0.5in, height: 10pt, color: #D6CD67)
#box(width: 0.5in, height: 10pt, color: #EDD466)
#box(width: 0.5in, height: 10pt, color: #E3BE62)

---
// Make sure that you can't do page related stuff in a box.
A
#box[
    B
    // Error: 16 cannot modify page from here
    #pagebreak()

    // Error: 11-15 cannot modify page from here
    #page("a4")
]
C

// No consequences from the page("A4") call here.
#pagebreak()
D
