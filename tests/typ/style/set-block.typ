// Test set in code blocks.

---
// Test that template in block is not affected by set
// rule in block ...
A{set text(fill: eastern); [B]}C

---
// ... no matter the order.
A{[B]; set text(fill: eastern)}C
