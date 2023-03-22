// Test citations and bibliographies.

---
// Error: 15-25 failed to parse biblatex file: wrong number of digits in line 5
#bibliography("/bad.bib")

---
// Test ambiguous reference.
= Introduction <arrgh>
// Error: 1-7 label occurs in the document and its bibliography
@arrgh
#bibliography("/works.bib")

---
#set page(width: 200pt)
= Details
See also #cite("arrgh", "distress", [p. 22]), @arrgh[p. 4], and @distress[p. 5].
#bibliography("/works.bib")

---
// Test unconventional order.
#set page(width: 200pt)
#bibliography("/works.bib", title: [Works to be cited], style: "author-date")
#line(length: 100%)
#[#set cite(brackets: false)
As described by @netwok],
the net-work is a creature of its own.
This is close to piratery! @arrgh
And quark! @quark
