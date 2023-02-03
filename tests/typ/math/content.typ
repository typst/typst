// Test arbitrary content in math.

---
// Test images and font fallback.
#let monkey = move(dy: 0.2em, image("/res/monkey.svg", height: 1em))
$ sum_(i=#emoji.apple)^#emoji.apple.red i + monkey/2 $

---
// Test table above fraction.
$ x := #table(columns: 2)[x][y]/mat(1, 2, 3) $

---
// Test non-formula math directly in content.
#math.attach($a$, top: [b])

---
// Test font switch.
#let here = text.with("Noto Sans")
$#here[f] := #here[Hi there]$.
