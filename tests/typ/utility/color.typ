// Test color creation functions.
// Ref: false

---
// Compare both ways.
#test(rgb(0.0, 0.3, 0.7), rgb("004db3"))

// Alpha channel.
#test(rgb(1.0, 0.0, 0.0, 0.5), rgb("ff000080"))

// Clamped.
#test(rgb(-30, 15.5, 0.5), rgb("00ff80"))

---
// Error: 6-11 invalid color
#rgb("lol")

---
// Error: 5-7 missing argument: red component
#rgb()

---
// Error: 5-11 missing argument: blue component
#rgb(0, 1)
