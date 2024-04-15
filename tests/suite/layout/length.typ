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
#test((500.934pt).pt(), 500.934)
#test((3.3453cm).cm(), 3.3453)
#test((4.3452mm).mm(), 4.3452)
#test((5.345in).inches(), 5.345)
#test((500.333666999pt).pt(), 500.333666999)
#test((3.5234354cm).cm(), 3.5234354)
#test((4.12345678mm).mm(), 4.12345678)
#test((5.333666999in).inches(), 5.333666999)
#test((4.123456789123456mm).mm(), 4.123456789123456)
#test((254cm).mm(), 2540.0)
#test(calc.round((254cm).inches(), digits: 2), 100.0)
#test((2540mm).cm(), 254.0)
#test(calc.round((2540mm).inches(), digits: 2), 100.0)
#test((100in).pt(), 7200.0)
#test(calc.round((100in).cm(), digits: 2), 254.0)
#test(calc.round((100in).mm(), digits: 2), 2540.0)
#test(5em.abs.cm(), 0.0)
#test((5em + 6in).abs.inches(), 6.0)

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
// Error: 1:17-1:19 expected length, found integer: a length needs a unit - did you mean 12pt?
#set text(size: 12)

--- length-ignore-em-pt-hint ---
// Error: 2-21 cannot convert a length with non-zero em units (`-6pt + 10.5em`) to pt
// Hint: 2-21 use `length.abs.pt()` instead to ignore its em component
#(10.5em - 6pt).pt()

--- length-ignore-em-cm-hint ---
// Error: 2-12 cannot convert a length with non-zero em units (`3em`) to cm
// Hint: 2-12 use `length.abs.cm()` instead to ignore its em component
#(3em).cm()

--- length-ignore-em-mm-hint ---
// Error: 2-20 cannot convert a length with non-zero em units (`-226.77pt + 93em`) to mm
// Hint: 2-20 use `length.abs.mm()` instead to ignore its em component
#(93em - 80mm).mm()

--- length-ignore-em-inches-hint ---
// Error: 2-24 cannot convert a length with non-zero em units (`432pt + 4.5em`) to inches
// Hint: 2-24 use `length.abs.inches()` instead to ignore its em component
#(4.5em + 6in).inches()
