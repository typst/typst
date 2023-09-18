// Test method calls.
// Ref: false

---
// Test whitespace around dot.
#test( "Hi there" . split() , ("Hi", "there"))

---
// Test mutating indexed value.
#{
  let matrix = (((1,), (2,)), ((3,), (4,)))
  matrix.at(1).at(0).push(5)
  test(matrix, (((1,), (2,)), ((3, 5), (4,))))
}

---
// Test multiline chain in code block.
#{
  let rewritten = "Hello. This is a sentence. And one more."
    .split(".")
    .map(s => s.trim())
    .filter(s => s != "")
    .map(s => s + "!")
    .join("\n ")

  test(rewritten, "Hello!\n This is a sentence!\n And one more!")
}

---
// Test .at() default values for content.
#test(auto, [a].at("doesn't exist", default: auto))

---
// Error: 2:10-2:13 type array has no method `fun`
#let numbers = ()
#numbers.fun()

---
// Error: 2:2-2:43 cannot mutate a temporary value
#let numbers = (1, 2, 3)
#numbers.map(v => v / 2).sorted().map(str).remove(4)

---
// Error: 2:3-2:19 cannot mutate a temporary value
#let numbers = (1, 2, 3)
#(numbers.sorted() = 1)

---
// Error: 2-5 cannot mutate a constant: box
#box.push(1)

---
// Test content fields method.
#test([a].fields(), (text: "a"))
#test([a *b*].fields(),  (children: ([a], [ ], strong[b])))

---
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

---
// Error: 2-21 cannot convert a length with non-zero em units (`-6pt + 10.5em`) to pt
// Hint: 2-21 use `length.abs.pt()` instead to ignore its em component
#(10.5em - 6pt).pt()

---
// Error: 2-12 cannot convert a length with non-zero em units (`3em`) to cm
// Hint: 2-12 use `length.abs.cm()` instead to ignore its em component
#(3em).cm()

---
// Error: 2-20 cannot convert a length with non-zero em units (`-226.77pt + 93em`) to mm
// Hint: 2-20 use `length.abs.mm()` instead to ignore its em component
#(93em - 80mm).mm()

---
// Error: 2-24 cannot convert a length with non-zero em units (`432pt + 4.5em`) to inches
// Hint: 2-24 use `length.abs.inches()` instead to ignore its em component
#(4.5em + 6in).inches()

---
// Test color kind method.
#test(rgb(1, 2, 3, 4).kind(), rgb)
#test(cmyk(4%, 5%, 6%, 7%).kind(), cmyk)
#test(luma(40).kind(), luma)
#test(rgb(1, 2, 3, 4).kind() != luma, true)

---
// Test color '.components()' without conversions
#test(repr(rgb(1, 2, 3, 4).components()), repr((1, 2, 3, 1.57%)))
#test(rgb(1, 2, 3).components(), (1, 2, 3, 100%))
#test(repr(luma(40).components()), repr((15.69%, )))
#test(repr(cmyk(4%, 5%, 6%, 7%).components()), repr((4%, 5%, 6%, 7%)))
#test(oklab(10%, 0.2, 0.3).components(), (10%, 0.2, 0.3, 100%))
#test(linear-rgb(10%, 20%, 30%).components(), (10%, 20%, 30%, 100%))
#test(hsv(10deg, 20%, 30%).components(), (10deg, 20%, 30%, 100%))
#test(hsl(10deg, 20%, 30%).components(), (10deg, 20%, 30%, 100%))

---
// Test color conversions.
#test(rgb(1, 2, 3).to-hex(), "#010203")
#test(rgb(1, 2, 3, 4).to-hex(), "#01020304")
#test(rgb(cmyk(4%, 5%, 6%, 7%)).components(), (228, 225, 223, 100%))
#test(cmyk(4%, 5%, 6%, 7%).to-hex(), "#e4e1df")
#test(rgb(luma(40)).components(false), (40, 40, 40))
#test(luma(40).to-hex(), "#282828")
#test(repr(cmyk(luma(40)).components()), repr((11.76%, 10.67%, 10.51%, 14.12%)))
#test(repr(hsl(luma(40))), repr(hsl(0deg, 0%, 15.69%)))
#test(repr(hsv(luma(40))), repr(hsv(0deg, 0%, 15.69%)))
#test(repr(linear-rgb(luma(40))), repr(linear-rgb(2.12%, 2.12%, 2.12%)))
#test(repr(linear-rgb(rgb(1, 2, 3))), repr(linear-rgb(0.03%, 0.06%, 0.09%)))
#test(repr(hsl(rgb(1, 2, 3))), repr(hsl(-150deg, 50%, 0.78%)))
#test(repr(hsv(rgb(1, 2, 3))), repr(hsv(-150deg, 66.67%, 1.18%)))
#test(repr(cmyk(rgb(1, 2, 3))), repr(cmyk(66.67%, 33.33%, 0%, 98.82%)))
#test(repr(luma(rgb(1, 2, 3))), repr(luma(0.73%)))

#let col = oklab(luma(40)).components()
#test((
  calc.round(col.at(0) / 100% * 1000) / 1000,
  calc.round(col.at(1) * 1000) / 1000,
  calc.round(col.at(2) * 1000) / 1000,
), (0.277, 0.0, 0.0))

#let col = oklab(rgb(1, 2, 3)).components()
#test((
  calc.round(col.at(0) / 100% * 1000) / 1000,
  calc.round(col.at(1) * 1000) / 1000,
  calc.round(col.at(2) * 1000) / 1000,
), (0.082, -0.004, -0.007))

---
// Test alignment methods.
#test(start.axis(), "horizontal")
#test(end.axis(), "horizontal")
#test(left.axis(), "horizontal")
#test(right.axis(), "horizontal")
#test(center.axis(), "horizontal")
#test(top.axis(), "vertical")
#test(bottom.axis(), "vertical")
#test(horizon.axis(), "vertical")
#test(start.inv(), end)
#test(end.inv(), start)
#test(left.inv(), right)
#test(right.inv(), left)
#test(center.inv(), center)
#test(top.inv(), bottom)
#test(bottom.inv(), top)
#test(horizon.inv(), horizon)

---
// Test 2d alignment methods.
#test((start + top).inv(), (end + bottom))
#test((end + top).inv(), (start + bottom))
#test((left + top).inv(), (right + bottom))
#test((right + top).inv(), (left + bottom))
#test((center + top).inv(), (center + bottom))
#test((start + bottom).inv(), (end + top))
#test((end + bottom).inv(), (start + top))
#test((left + bottom).inv(), (right + top))
#test((right + bottom).inv(), (left + top))
#test((center + bottom).inv(), (center + top))
#test((start + horizon).inv(), (end + horizon))
#test((end + horizon).inv(), (start + horizon))
#test((left + horizon).inv(), (right + horizon))
#test((right + horizon).inv(), (left + horizon))
#test((center + horizon).inv(), (center + horizon))
#test((top + start).inv(), (end + bottom))
#test((bottom + end).inv(), (start + top))
#test((horizon + center).inv(), (center + horizon))

---
// Test direction methods.
#test(ltr.axis(), "horizontal")
#test(rtl.axis(), "horizontal")
#test(ttb.axis(), "vertical")
#test(btt.axis(), "vertical")
#test(ltr.start(), left)
#test(rtl.start(), right)
#test(ttb.start(), top)
#test(btt.start(), bottom)
#test(ltr.end(), right)
#test(rtl.end(), left)
#test(ttb.end(), bottom)
#test(btt.end(), top)
#test(ltr.inv(), rtl)
#test(rtl.inv(), ltr)
#test(ttb.inv(), btt)
#test(btt.inv(), ttb)

---
// Test angle methods.
#test(1rad.rad(), 1.0)
#test(1.23rad.rad(), 1.23)
#test(0deg.rad(), 0.0)
#test(2deg.deg(), 2.0)
#test(2.94deg.deg(), 2.94)
#test(0rad.deg(), 0.0)

---
// Test date methods.
#test(datetime(day: 1, month: 1, year: 2000).ordinal(), 1);
#test(datetime(day: 1, month: 3, year: 2000).ordinal(), 31 + 29 + 1);
#test(datetime(day: 31, month: 12, year: 2000).ordinal(), 366);
#test(datetime(day: 1, month: 3, year: 2001).ordinal(), 31 + 28 + 1);
#test(datetime(day: 31, month: 12, year: 2001).ordinal(), 365);
