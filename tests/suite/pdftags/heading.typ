--- heading-tags-basic pdftags ---
= Level 1
== Level 2
=== Level 3

--- heading-tags-first-is-not-level-1 pdftags ---
// Error: 1-11 PDF/UA-1 error: the first heading must be of level 1
== Level 2

--- heading-tags-non-consecutive-levels pdftags ---
// Error: 2:1-2:12 PDF/UA-1 error: skipped from heading level 1 to 3
// Hint: 2:1-2:12 heading levels must be consecutive
= Level 1
=== Level 3

--- heading-tags-complex pdftags ---
= Level 1
== Level 2
=== Level 3
=== Level 3
== Level 2
=== Level 3
==== Level 4
== Level 2
=== Level 3
=== Level 3
=== Level 3
= Level 1
== Level 2
