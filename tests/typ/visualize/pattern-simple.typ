// Tests that simple patterns work.

---
#set page(width: auto, height: auto, margin: 0pt)
#let pat = pattern(size: (10pt, 10pt), line(stroke: 4pt, start: (0%, 0%), end: (100%, 100%)))
#rect(width: 50pt, height: 50pt, fill: pat)

---
#set page(width: auto, height: auto, margin: 0pt)

#let pat = pattern(size: (10pt, 10pt), {
    place(line(stroke: 4pt, start: (0%, 0%), end: (100%, 100%)))
    place(line(stroke: 4pt, start: (100%,0%), end: (200%, 100%)))
    place(line(stroke: 4pt, start: (0%,100%), end: (100%, 200%)))
    place(line(stroke: 4pt, start: (-100%,0%), end: (0%, 100%)))
    place(line(stroke: 4pt, start: (0%,-100%), end: (100%, 0%)))
})
#rect(width: 50pt, height: 50pt, fill: pat)
