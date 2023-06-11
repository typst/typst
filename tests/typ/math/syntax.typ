// Test math syntax.

---
// Test Unicode math.
$ ∑_(i=0)^ℕ a ∘ b = \u{2211}_(i=0)^NN a compose b $

---
// Test a few shorthands.
$ underline(f' : NN -> RR) \
  n |-> cases(
    [|1|] &"if" n >>> 10,
    2 * 3 &"if" n != 5,
    1 - 0 thick &...,
  ) $

---
// Test common symbols.
$ dot \ dots \ ast \ tilde \ star $

---
// Error: 1-2 unclosed delimiter
$a
