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
// Error: 2:2-2:15 type array has no method `fun`
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
// Test length unit conversions.
#test((3.345cm).cm(), 3.345)
#test((4.345mm).mm(), 4.345)
#test((5.345in).inches(), 5.345)
#test((3.5234354cm).cm(), 3.5234354)
#test((4.12345678mm).mm(), 4.12345678)
#test((5.333666999in).inches(), 5.333666999)
#test((3.5234354cm).cm(digits: 0), 3.0)
#test((3.5234354cm).cm(digits: 7), 3.5234354)
#test((4.12345678mm).mm(digits: 8), 4.12345678)
#test((5.333666999in).inches(digits: 9), 5.333666999)
#test((4.123456789123456mm).mm(), 4.1234567891)
#test((4.123456789123456mm).mm(digits: 9), 4.123456789)
#test((4.123456789123456mm).mm(digits: 15), 4.123456789123456)

---
// Error: 27-29 number must be at least zero
#(3.345cm).inches(digits: -1)

---
// Test color kind method.
#test(rgb(1, 2, 3, 4).kind(), "rgba")
#test(cmyk(4%, 5%, 6%, 7%).kind(), "cmyk")
#test(luma(40).kind(), "luma")

---
// Test color conversion methods.
#test(rgb(1, 2, 3).hex(), "#010203")
#test(rgb(1, 2, 3, 4).hex(), "#01020304")
#test(cmyk(4%, 5%, 6%, 7%).to-rgba().kind(), "rgba")
#test(cmyk(4%, 5%, 6%, 7%).to-rgba().values, (228, 225, 223, 255))
#test(cmyk(4%, 5%, 6%, 7%).hex(), "#e4e1df")
#test(luma(40).to-rgba().kind(), "rgba")
#test(luma(40).to-rgba().values, (40, 40, 40, 255))
#test(luma(40).hex(), "#282828")
#test(luma(40).to-cmyk().kind(), "cmyk")
#test(repr(luma(40).to-cmyk().values), repr((11.8%, 10.6%, 10.6%, 14.1%)))

---
// Error: 2-27 cannot convert color kind 'rgba' to 'cmyk'
#rgb(1, 2, 3, 4).to-cmyk()

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
#test(start.inverse(), end)
#test(end.inverse(), start)
#test(left.inverse(), right)
#test(right.inverse(), left)
#test(center.inverse(), center)
#test(top.inverse(), bottom)
#test(bottom.inverse(), top)
#test(horizon.inverse(), horizon)

---
// Test 2d alignment methods.
#test((start + top).inverse(), (end + bottom))
#test((end + top).inverse(), (start + bottom))
#test((left + top).inverse(), (right + bottom))
#test((right + top).inverse(), (left + bottom))
#test((center + top).inverse(), (center + bottom))
#test((start + bottom).inverse(), (end + top))
#test((end + bottom).inverse(), (start + top))
#test((left + bottom).inverse(), (right + top))
#test((right + bottom).inverse(), (left + top))
#test((center + bottom).inverse(), (center + top))
#test((start + horizon).inverse(), (end + horizon))
#test((end + horizon).inverse(), (start + horizon))
#test((left + horizon).inverse(), (right + horizon))
#test((right + horizon).inverse(), (left + horizon))
#test((center + horizon).inverse(), (center + horizon))
#test((top + start).inverse(), (end + bottom))
#test((bottom + end).inverse(), (start + top))
#test((horizon + center).inverse(), (center + horizon))

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
#test(ltr.inverse(), rtl)
#test(rtl.inverse(), ltr)
#test(ttb.inverse(), btt)
#test(btt.inverse(), ttb)
