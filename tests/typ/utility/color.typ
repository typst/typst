// Test color creation functions.
// Ref: false

---
// Compare both ways.
#test(rgb(0%, 30%, 70%), rgb("004db3"))

// Alpha channel.
#test(rgb(255, 0, 0, 50%), rgb("ff000080"))

// Test color modification methods.
#test(rgb(25, 35, 45).lighten(10%), rgb(48, 57, 66))
#test(rgb(40, 30, 20).darken(10%), rgb(36, 27, 18))
#test(rgb("#133337").negate(), rgb(236, 204, 200))
#test(white.lighten(100%), white)

---
// Test gray color conversion.
// Ref: true
#rect(fill: luma(0))
#rect(fill: luma(80%))

---
// Test gray color modification.
#test(luma(20%).lighten(50%), luma(60%))
#test(luma(80%).darken(20%), luma(63.9%))
#test(luma(80%).negate(), luma(20%))

---
// Test CMYK color conversion.
// Ref: true
#let c = cmyk(50%, 64%, 16%, 17%)
#rect(width: 1cm, fill: cmyk(69%, 11%, 69%, 41%))
#rect(width: 1cm, fill: c)
#rect(width: 1cm, fill: c.negate())

#for x in range(0, 11) {
    square(width: 9pt, fill: c.lighten(x * 10%))
}
#for x in range(0, 11) {
    square(width: 9pt, fill: c.darken(x * 10%))
}

---
// Error for values that are out of range.
// Error: 11-14 must be between 0 and 255
#test(rgb(-30, 15, 50))

---
// Error: 6-11 string contains non-hexadecimal letters
#rgb("lol")

---
// Error: 5-7 missing argument: red component
#rgb()

---
// Error: 5-11 missing argument: blue component
#rgb(0, 1)

---
// Error: 21-26 expected integer or ratio, found boolean
#rgb(10%, 20%, 30%, false)
