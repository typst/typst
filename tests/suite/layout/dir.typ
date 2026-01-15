--- dir-from eval ---
#test(direction.from(left), ltr)
#test(direction.from(right), rtl)
#test(direction.from(top), ttb)
#test(direction.from(bottom), btt)

--- dir-from-invalid paged ---
// Error: 17-23 cannot convert this alignment to a side
#direction.from(center)

--- dir-to eval ---
#test(direction.to(left), rtl)
#test(direction.to(right), ltr)
#test(direction.to(top), btt)
#test(direction.to(bottom), ttb)

--- dir-to-invalid paged ---
// Error: 15-21 cannot convert this alignment to a side
#direction.to(center)

--- dir-axis eval ---
#test(ltr.axis(), "horizontal")
#test(rtl.axis(), "horizontal")
#test(ttb.axis(), "vertical")
#test(btt.axis(), "vertical")

--- dir-sign eval ---
#test(ltr.sign(), 1)
#test(rtl.sign(), -1)
#test(ttb.sign(), 1)
#test(btt.sign(), -1)

--- dir-start eval ---
#test(ltr.start(), left)
#test(rtl.start(), right)
#test(ttb.start(), top)
#test(btt.start(), bottom)

--- dir-end eval ---
#test(ltr.end(), right)
#test(rtl.end(), left)
#test(ttb.end(), bottom)
#test(btt.end(), top)

--- dir-inv eval ---
#test(ltr.inv(), rtl)
#test(rtl.inv(), ltr)
#test(ttb.inv(), btt)
#test(btt.inv(), ttb)
