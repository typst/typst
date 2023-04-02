// Test paths.

---
#set page(height: 500pt, width: 500pt)
#set path(stroke: black + 1pt)

#path(fill: red, stroke: none, closed: true, ((0%, 0%), (2%, -2%)), ((10%, 10%), (2%, -2%)), ((0%, 10%), (2%, 2%)), ((10%, 0%), (2%, 2%)))
#path(fill: purple, (0%, 0%), (10%, 10%), (0%, 10%), (10%, 0%))
#path(fill: blue, closed: true, ((50%, 0%), (15%, 15%), (-10%, 0%)), ((50%, 30%), (-10%, 0%), (0%, 0%)), ((65%, 15%), (60%, -15%), (60%, 0%)))

---
// Error: 7-9 path vertex must be 1, 2, or 3 points
#path(())

---
// Error: 7-47 path vertex must be 1, 2, or 3 points
#path(((0%, 0%), (0%, 0%), (0%, 0%), (0%, 0%)))

---
// Error: 7-31 point array must contain exactly two entries
#path(((0%, 0%), (0%, 0%, 0%)))