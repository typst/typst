--- length-fields ---
// Test length fields.
#test((1pt).em, 0.0)
#test((1pt).abs, 1pt)
#test((3em).em, 3.0)
#test((3em).abs, 0pt)
#test((2em + 2pt).em, 2.0)
#test((2em + 2pt).abs, 2pt)

--- length-to-unit ---
// Test length unit conversions.
#let t(a, b) = assert(calc.abs(a - b) < 1e-6)

#t((500.934pt).pt(), 500.934)
#t((3.3453cm).cm(), 3.3453)
#t((4.3452mm).mm(), 4.3452)
#t((5.345in).inches(), 5.345)
#t((500.333666999pt).pt(), 500.333666999)
#t((3.523435cm).cm(), 3.523435)
#t((4.12345678mm).mm(), 4.12345678)
#t((5.333666999in).inches(), 5.333666999)
#t((4.123456789123456mm).mm(), 4.123456789123456)
#t((254cm).mm(), 2540.0)
#t((254cm).inches(), 100.0)
#t((2540mm).cm(), 254.0)
#t((2540mm).inches(), 100.0)
#t((100in).pt(), 7200.0)
#t((100in).cm(), 254.0)
#t((100in).mm(), 2540.0)
#t(5em.abs.cm(), 0.0)
#t((5em + 6in).abs.inches(), 6.0)

--- length-to-absolute ---
// Test length `to-absolute` method.
#set text(size: 12pt)
#context {
  test((6pt).to-absolute(), 6pt)
  test((6pt + 10em).to-absolute(), 126pt)
  test((10em).to-absolute(), 120pt)
}

#set text(size: 64pt)
#context {
  test((6pt).to-absolute(), 6pt)
  test((6pt + 10em).to-absolute(), 646pt)
  test((10em).to-absolute(), 640pt)
}

--- length-unit-hint ---
// Error: 17-19 expected length, found integer
// Hint: 17-19 a length needs a unit - did you mean 12pt?
#set text(size: 12)

--- length-ignore-em-pt-hint ---
// Error: 2-21 cannot convert a length with non-zero em units (`-6pt + 10.5em`) to pt
// Hint: 2-21 use `length.to-absolute()` to resolve its em component (requires context)
// Hint: 2-21 or use `length.abs.pt()` instead to ignore its em component
#(10.5em - 6pt).pt()

--- length-ignore-em-cm-hint ---
// Error: 2-12 cannot convert a length with non-zero em units (`3em`) to cm
// Hint: 2-12 use `length.to-absolute()` to resolve its em component (requires context)
// Hint: 2-12 or use `length.abs.cm()` instead to ignore its em component
#(3em).cm()

--- length-ignore-em-mm-hint ---
// Error: 2-20 cannot convert a length with non-zero em units (`-226.77pt + 93em`) to mm
// Hint: 2-20 use `length.to-absolute()` to resolve its em component (requires context)
// Hint: 2-20 or use `length.abs.mm()` instead to ignore its em component
#(93em - 80mm).mm()

--- length-ignore-em-inches-hint ---
// Error: 2-24 cannot convert a length with non-zero em units (`432pt + 4.5em`) to inches
// Hint: 2-24 use `length.to-absolute()` to resolve its em component (requires context)
// Hint: 2-24 or use `length.abs.inches()` instead to ignore its em component
#(4.5em + 6in).inches()
