// Test enums.

---
. Embrace
. Extend
. Extinguish

---
1. First.
   2. Second.

1. Back to first.

---
2. Second
1. First
  . Indented

---
// Test automatic numbering in summed templates.
#for i in range(5) {
   [. #roman(1 + i)]
}
