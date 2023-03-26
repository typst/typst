// Test bezier curves.

---
#set page(height: 250pt, width: 250pt)
#box[
    #show bezier: place.with(top + left)
    #bezier(end: (10%, 10%))
    #bezier(start: (10%, 0%), end: (20%, 10%), start-control-point: (0%, 10%)) // |
    #bezier(start: (20%, 0%), end: (30%, 10%), end-control-point: (-10%, 0%)) //  | These two should be the same line
    #bezier(start: (30%, 0%), end: (40%, 10%), start-control-point: (0%, 10%), end-control-point: (-10%, 0%))
    #bezier(start: (40%, 0%), end: (40%, 0%), start-control-point: (20%, 0%), end-control-point: (0%, 20%))
]

---
// Test errors.

// Error: 14-21 point array must contain exactly two entries
#bezier(end: (50pt,))

---
// Error: 16-28 expected relative length, found angle
#bezier(start: (3deg, 10pt))

---
// Error: 14-26 expected relative length, found angle
#bezier(end: (3deg, 10pt))

---
// Error: 30-42 expected relative length, found angle
#bezier(start-control-point: (3deg, 10pt))