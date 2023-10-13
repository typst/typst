// Test under/over things.

---
// Test braces.
$ x = underbrace(
  1 + 2 + ... + 5,
  underbrace("numbers", x + y)
) $

---
// Test lines and brackets.
$ x = overbracket(
  overline(underline(x + y)),
  1 + 2 + ... + 5,
) $

---
// Test brackets.
$ underbracket([1, 2/3], "relevant stuff")
          arrow.l.r.double.long
  overbracket([4/5,6], "irrelevant stuff") $
