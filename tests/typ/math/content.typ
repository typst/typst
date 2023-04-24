// Test arbitrary content in math.

---
// Test images and font fallback.
#let monkey = move(dy: 0.2em, image("/monkey.svg", height: 1em))
$ sum_(i=#emoji.apple)^#emoji.apple.red i + monkey/2 $

---
// Test tables.
$ x := #table(columns: 2)[x][y]/mat(1, 2, 3)
     = #table[A][B][C] $
---
// Test non-equation math directly in content.
#math.attach($a$, t: [b])

---
// Test font switch.
#let here = text.with(font: "Noto Sans")
$#here[f] := #here[Hi there]$.
