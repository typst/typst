// Flow layout should terminate!
// https://github.com/typst/typst/issues/3641
//
// This is not yet ideal: The heading should not move to the second page, but
// that's a separate bug and not a regression.

---
#set page(height: 40pt)

= Heading
#lorem(6)
