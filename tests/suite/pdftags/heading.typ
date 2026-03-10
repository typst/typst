--- heading-tags-basic pdftags pdfstandard(ua-1) ---
= Level 1
== Level 2
=== Level 3

--- heading-tags-first-is-not-level-1 pdftags pdfstandard(ua-1) ---
// Error: 1-11 PDF/UA-1 error: the first heading must be of level 1
== Level 2

--- heading-tags-non-consecutive-levels pdftags pdfstandard(ua-1) ---
// Error: 2:1-2:12 PDF/UA-1 error: skipped from heading level 1 to 3
// Hint: 2:1-2:12 heading levels must be consecutive
= Level 1
=== Level 3

--- heading-tags-complex pdftags pdfstandard(ua-1) ---
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

--- heading-tags-empty pdftags pdfstandard(ua-1) ---
// Error: 1-2 PDF/UA-1 error: heading title is empty
=

--- heading-tags-context-body pdftags pdfstandard(ua-1) ---
// Error: 2-32 PDF/UA-1 error: heading title could not be determined
// Hint: 2-32 this seems to be caused by a context expression within the heading
// Hint: 2-32 consider wrapping the entire heading in a context expression instead
#heading(context [Hello there])
