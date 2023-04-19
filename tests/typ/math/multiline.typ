// Test multiline math.

---
// Test basic alignment.
$ x &= x + y \
    &= x + 2z \
    &= sum x dot 2z $

---
// Test text before first alignment point.
$ x + 1 &= a^2 + b^2 \
      y &= a + b^2 \
      z &= alpha dot beta $

---
// Test space between inner alignment points.
$ a + b &= 2 + 3 &= 5 \
      b &= c     &= 3 $

---
// Test in case distinction.
$ f := cases(
  1 + 2 &"iff" &x,
  3     &"if"  &y,
) $

---
// Test mixing lines with and some without alignment points.
$ "abc" &= c \
   &= d + 1 \
   = x $

---
// Test multiline subscript.
$ sum_(n in NN \ n <= 5) n = (5(5+1))/2 = 15 $

---
// Test no trailing line break.
$
"abc" &= c
$
No trailing line break.

---
// Test single trailing line break.
$
"abc" &= c \
$
One trailing line break.

---
// Test multiple trailing line breaks.
$
"abc" &= c \ \ \
$
Multiple trailing line breaks.
