// Test return value of for loops.

---
// Template body yields template.
// Should output `234`.
#for v in (1, 2, 3, 4) [#if v >= 2 [{v}]]

---
// Block body yields template.
// Should output `[1st, 2nd, 3rd, 4th, 5th, 6th]`.
{
    "[" + for v in (1, 2, 3, 4, 5, 6) {
        (if v > 1 [, ]
            + [{v}]
            + if v == 1 [st]
            + if v == 2 [nd]
            + if v == 3 [rd]
            + if v >= 4 [th])
     } + "]"
}
