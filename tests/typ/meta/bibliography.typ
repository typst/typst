// Test citations and bibliographies.

---
// Test ambiguous reference.
= Introduction <arrgh>
// Error: 1-7 label occurs in the document and its bibliography
@arrgh
#bibliography("/files/works.bib")

---
#set page(width: 200pt)
= Details
See also #cite("arrgh", "distress", supplement: [p. 22]), @arrgh[p. 4], and @distress[p. 5].
#bibliography("/files/works.bib")

---
// Test unconventional order.
#set page(width: 200pt)
#bibliography(
  "/files/works.bib",
  title: [Works to be cited],
  style: "chicago-author-date",
)
#line(length: 100%)

#[#set cite(brackets: false)
As described by @netwok],
the net-work is a creature of its own.
This is close to piratery! @arrgh
And quark! @quark

---
// Error: 15-55 duplicate bibliography keys: arrgh, distress, glacier-melt, issue201, mcintosh_anxiety, netwok, psychology25, quark, restful, sharing, tolkien54
#bibliography(("/files/works.bib", "/files/works.bib"))

---
#set page(width: 200pt)
#set heading(numbering: "1.")
#show bibliography: set heading(numbering: "1.")

= Multiple Bibs
Now we have multiple bibliographies containing #cite("glacier-melt", "keshav2007read")
#bibliography(("/files/works.bib", "/files/works_too.bib"))
