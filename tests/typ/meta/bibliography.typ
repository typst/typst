// Test citations and bibliographies.

---
#set page(width: 200pt)

= Details
See also @arrgh #cite(<distress>, supplement: [p.~22]), @arrgh[p.~4], and @distress[p.~5].
#bibliography("/assets/bib/works.bib")

---
// Test unconventional order.
#set page(width: 200pt)
#bibliography(
  "/assets/bib/works.bib",
  title: [Works to be cited],
  style: "chicago-author-date",
)
#line(length: 100%)

As described by #cite(<netwok>, form: "prose"),
the net-work is a creature of its own.
This is close to piratery! @arrgh
And quark! @quark

---
#set page(width: 200pt)
#set heading(numbering: "1.")
#show bibliography: set heading(numbering: "1.")

= Multiple Bibs
Now we have multiple bibliographies containing @glacier-melt @keshav2007read
#bibliography(("/assets/bib/works.bib", "/assets/bib/works_too.bib"))

---
// Test ambiguous reference.
= Introduction <arrgh>

// Error: 1-7 label occurs in the document and its bibliography
@arrgh
#bibliography("/assets/bib/works.bib")

---
// Error: 15-65 duplicate bibliography keys: netwok, issue201, arrgh, quark, distress, glacier-melt, tolkien54, DBLP:books/lib/Knuth86a, sharing, restful, mcintosh_anxiety, psychology25
#bibliography(("/assets/bib/works.bib", "/assets/bib/works.bib"))
