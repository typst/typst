// Test citations and bibliographies.

--- bibliography-basic ---
#set page(width: 200pt)

= Details
See also @arrgh #cite(<distress>, supplement: [p.~22]), @arrgh[p.~4], and @distress[p.~5].
#bibliography("/assets/bib/works.bib")

--- bibliography-before-content ---
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

--- bibliography-multiple-files ---
#set page(width: 200pt)
#set heading(numbering: "1.")
#show bibliography: set heading(numbering: "1.")

= Multiple Bibs
Now we have multiple bibliographies containing @glacier-melt @keshav2007read
#bibliography(("/assets/bib/works.bib", "/assets/bib/works_too.bib"))

--- bibliography-duplicate-key ---
// Error: 15-65 duplicate bibliography keys: netwok, issue201, arrgh, quark, distress, glacier-melt, tolkien54, DBLP:books/lib/Knuth86a, sharing, restful, mcintosh_anxiety, psychology25
#bibliography(("/assets/bib/works.bib", "/assets/bib/works.bib"))

--- bibliography-ordering ---
#set page(width: 300pt)

@mcintosh_anxiety
@psychology25

#bibliography("/assets/bib/works.bib")

--- bibliography-full ---
#set page(paper: "a6", height: auto)
#bibliography("/assets/bib/works_too.bib", full: true)

--- bibliography-math ---
#set page(width: 200pt)

@Zee04
#bibliography("/assets/bib/works_too.bib", style: "mla")

--- bibliography-grid-par ---
// Ensure that a grid-based bibliography does not produce paragraphs.
#show par: highlight

@Zee04
@keshav2007read

#bibliography("/assets/bib/works_too.bib")

--- bibliography-indent-par ---
// Ensure that an indent-based bibliography does not produce paragraphs.
#show par: highlight

@Zee04
@keshav2007read

#bibliography("/assets/bib/works_too.bib", style: "mla")

--- bibliography-style-not-suitable ---
// Error: 2-62 CSL style "Alphanumeric" is not suitable for bibliographies
#bibliography("/assets/bib/works.bib", style: "alphanumeric")

--- bibliography-empty-key ---
#let src = ```yaml
"":
  type: Book
```
// Error: 15-30 bibliography contains entry with empty key
#bibliography(bytes(src.text))

--- issue-4618-bibliography-set-heading-level ---
// Test that the bibliography block's heading is set to 2 by the show rule,
// and therefore should be rendered like a level-2 heading. Notably, this
// bibliography heading should not be underlined.
#show heading.where(level: 1): it => [ #underline(it.body) ]
#show bibliography: set heading(level: 2)

= Level 1
== Level 2
@Zee04

#bibliography("/assets/bib/works_too.bib")
