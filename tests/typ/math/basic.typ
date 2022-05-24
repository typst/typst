// Test math formulas.

---
The sum of $a$ and $b$ is $a + b$.

---
We will show that:
$[ a^2 + b^2 = c^2 ]$

---
Prove by induction:
$[ \sum_{k=0}^n k = \frac{n(n+1)}{2} ]$

---
// Test that blackboard style looks nice.
$[ f: \mathbb{N} \rightarrow \mathbb{R} ]$

---
#set math(family: "IBM Plex Sans")

// Error: 1-4 font is not suitable for math
$a$

---
// Error: 1-10 expected '}' found EOF
$\sqrt{x$

---
// Error: 2:1 expected closing bracket and dollar sign
$[a
