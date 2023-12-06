// Test citations and bibliographies.

---
#set page(width: 200pt)

= Details
See also @arrgh #cite(<distress>, supplement: [p.~22]), @arrgh[p.~4], and @distress[p.~5].
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
#bibliography(("/files/works.bib", "/files/works_too.bib"))

---
// Test ambiguous reference.
= Introduction <arrgh>

// Error: 1-7 label occurs in the document and its bibliography
@arrgh
#bibliography("/files/works.bib")

---
// Error: 15-55 duplicate bibliography keys: netwok, issue201, arrgh, quark, distress, glacier-melt, tolkien54, sharing, restful, mcintosh_anxiety, psychology25
#bibliography(("/files/works.bib", "/files/works.bib"))
