// Test color creation functions.
// Ref: false

---
// Compare both ways.
#test(rgb(0%, 30%, 70%), rgb("004db3"))

// Alpha channel.
#test(rgb(255, 0, 0, 50%), rgb("ff000080"))

---
// Test CMYK color conversion.
// Ref: true
#rect(fill: cmyk(69%, 11%, 69%, 41%))
#rect(fill: cmyk(50%, 64%, 16%, 17%))

---
// Error for values that are out of range.
// Error: 11-14 must be between 0 and 255
#test(rgb(-30, 15, 50))

---
// Error: 6-11 invalid hex string
#rgb("lol")

---
// Error: 5-7 missing argument: red component
#rgb()

---
// Error: 5-11 missing argument: blue component
#rgb(0, 1)

---
// Error: 21-26 expected integer or relative, found boolean
#rgb(10%, 20%, 30%, false)
