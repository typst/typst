// Test creation and conversion functions.
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

// Color mixing, in Oklab space by default.
#test(color.mix(rgb("#ff0000"), rgb("#00ff00")).to-rgba(), rgb("#d0a800"))
#test(color.mix(rgb("#ff0000"), rgb("#00ff00"), space: "oklab").to-rgba(), rgb("#d0a800"))
#test(color.mix(rgb("#ff0000"), rgb("#00ff00"), space: "srgb").to-rgba(), rgb("#808000"))

#test(color.mix(red, green, blue).to-rgba(), rgb("#909282"))
#test(color.mix(red, blue, green).to-rgba(), rgb("#909282"))
#test(color.mix(blue, red, green).to-rgba(), rgb("#909282"))

// Mix with weights.
#test(color.mix((red, 50%), (green, 50%)).to-rgba(), rgb("#c0983b"))
#test(color.mix((red, 0.5), (green, 0.5)).to-rgba(), rgb("#c0983b"))
#test(color.mix((red, 5), (green, 5)).to-rgba(), rgb("#c0983b"))
#test(color.mix((green, 5), (white, 0), (red, 5)).to-rgba(), rgb("#c0983b"))
#test(color.mix((rgb("#aaff00"), 25%), (rgb("#aa00ff"), 75%), space: "srgb"), rgb("#aa40bf"))
#test(color.mix((rgb("#aaff00"), 50%), (rgb("#aa00ff"), 50%), space: "srgb"), rgb("#aa8080"))
#test(color.mix((rgb("#aaff00"), 75%), (rgb("#aa00ff"), 25%), space: "srgb"), rgb("#aabf40"))

---
// Test gray color conversion.
// Ref: true
#stack(dir: ltr, rect(fill: luma(0)), rect(fill: luma(80%)))

---
// Error for values that are out of range.
// Error: 11-14 number must be between 0 and 255
#test(rgb(-30, 15, 50))

---
// Error: 6-11 color string contains non-hexadecimal letters
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

---
// Error: 12-24 expected float or ratio, found string
// Error: 26-39 expected float or ratio, found string
#color.mix((red, "yes"), (green, "no"), (green, 10%))

---
// Error: 12-23 expected a color or color-weight pair
#color.mix((red, 1, 2))

---
// Error: 31-38 expected "oklab", "srgb", "d65-gray", "linear-rgb", "hsl", "hsv", or "cmyk"
#color.mix(red, green, space: "cyber")

---
// Ref: true
#let envelope = symbol(
  "🖂",
  ("stamped", "🖃"),
  ("stamped.pen", "🖆"),
  ("lightning", "🖄"),
  ("fly", "🖅"),
)

#envelope
#envelope.stamped
#envelope.pen
#envelope.stamped.pen
#envelope.lightning
#envelope.fly

---
// Error: 8-10 expected at least one variant
#symbol()

---
// Test conversion to string.
#test(str(123), "123")
#test(str(123, base: 3), "11120")
#test(str(-123, base: 16), "-7b")
#test(str(9223372036854775807, base: 36), "1y2p0ij32e8e7")
#test(str(50.14), "50.14")
#test(str(10 / 3).len() > 10, true)

---
// Error: 6-8 expected integer, float, bytes, label, type, or string, found content
#str([])

---
// Error: 17-19 base must be between 2 and 36
#str(123, base: 99)

---
// Error: 18-19 base is only supported for integers
#str(1.23, base: 2)

---
// Test the unicode function.
#test(str.from-unicode(97), "a")
#test(str.to-unicode("a"), 97)

---
// Error: 19-22 expected integer, found content
#str.from-unicode([a])

---
// Error: 17-21 expected exactly one character
#str.to-unicode("ab")

---
// Error: 19-21 number must be at least zero
#str.from-unicode(-1)

---
// Error: 18-28 0x110000 is not a valid codepoint
#str.from-unicode(0x110000) // 0x10ffff is the highest valid code point

---
#assert(range(2, 5) == (2, 3, 4))

---
// Test displaying of dates.
#test(datetime(year: 2023, month: 4, day: 29).display(), "2023-04-29")
#test(datetime(year: 2023, month: 4, day: 29).display("[year]"), "2023")
#test(
  datetime(year: 2023, month: 4, day: 29)
    .display("[year repr:last_two]"),
  "23",
)
#test(
  datetime(year: 2023, month: 4, day: 29)
    .display("[year] [month repr:long] [day] [week_number] [weekday]"),
  "2023 April 29 17 Saturday",
)

// Test displaying of times
#test(datetime(hour: 14, minute: 26, second: 50).display(), "14:26:50")
#test(datetime(hour: 14, minute: 26, second: 50).display("[hour]"), "14")
#test(
  datetime(hour: 14, minute: 26, second: 50)
    .display("[hour repr:12 padding:none]"),
  "2",
)
#test(
  datetime(hour: 14, minute: 26, second: 50)
    .display("[hour], [minute], [second]"), "14, 26, 50",
)

// Test displaying of datetimes
#test(
  datetime(year: 2023, month: 4, day: 29, hour: 14, minute: 26, second: 50).display(),
  "2023-04-29 14:26:50",
)

// Test getting the year/month/day etc. of a datetime
#let d = datetime(year: 2023, month: 4, day: 29, hour: 14, minute: 26, second: 50)
#test(d.year(), 2023)
#test(d.month(), 4)
#test(d.weekday(), 6)
#test(d.day(), 29)
#test(d.hour(), 14)
#test(d.minute(), 26)
#test(d.second(), 50)

#let e = datetime(year: 2023, month: 4, day: 29)
#test(e.hour(), none)
#test(e.minute(), none)
#test(e.second(), none)

// Test today
#test(datetime.today().display(), "1970-01-01")
#test(datetime.today(offset: auto).display(), "1970-01-01")
#test(datetime.today(offset: 2).display(), "1970-01-01")

---
// Error: 10-12 at least one of date or time must be fully specified
#datetime()

---
// Error: 10-42 time is invalid
#datetime(hour: 25, minute: 0, second: 0)

---
// Error: 10-41 date is invalid
#datetime(year: 2000, month: 2, day: 30)

---
// Error: 27-34 missing closing bracket for bracket at index 0
#datetime.today().display("[year")

---
// Error: 27-38 invalid component name 'nothing' at index 1
#datetime.today().display("[nothing]")

---
// Error: 27-50 invalid modifier 'wrong' at index 6
#datetime.today().display("[year wrong:last_two]")

---
// Error: 27-33 expected component name at index 2
#datetime.today().display("  []")

---
// Error: 2-36 failed to format datetime (insufficient information)
#datetime.today().display("[hour]")
