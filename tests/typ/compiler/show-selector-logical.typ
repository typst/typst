// Test and/or selectors in show rules.

---
// Looking forward to `heading.where(level: 1 | 2)` :)
#show heading.where(level: 1).or(heading.where(level: 2)): set text(red)
= L1
== L2
=== L3
==== L4

---
// Test element selector combined with label selector.
#show selector(strong).or(<special>): highlight
I am *strong*, I am _emphasized_, and I am #[special<special>].

---
// Ensure that text selector cannot be nested in and/or. That's too complicated,
// at least for now.

// Error: 7-41 this selector cannot be used with show
#show heading.where(level: 1).or("more"): set text(red)
