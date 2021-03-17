// Test shapes.

---
// Test `rect` function.

#page("a8", flip: true)

// Fixed width, should have text height.
#rect(width: 2cm, fill: #9650D6)[Legal]

Sometimes there is no box.

// Fixed height, should span line.
#rect(height: 1cm, width: 100%, fill: #734CED)[B]

// Empty with fixed width and height.
#rect(width: 6cm, height: 12pt, fill: #CB4CED)

// Not visible, but creates a gap between the boxes above and below.
#rect(width: 2in, fill: #ff0000)

// These are in a row!
#rect(width: 0.5in, height: 10pt, fill: #D6CD67)
#rect(width: 0.5in, height: 10pt, fill: #EDD466)
#rect(width: 0.5in, height: 10pt, fill: #E3BE62)

---
// Make sure that you can't do page related stuff in a shape.
A
#rect[
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
