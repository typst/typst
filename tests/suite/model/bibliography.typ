// Test citations and bibliographies.

--- bibliography-basic render html ---
#show: it => context { set page(width: 200pt) if target() == "paged"; it }

= Details
See also @arrgh #cite(<distress>, supplement: [p.~22]), @arrgh[p.~4], and @distress[p.~5].
#bibliography("/assets/bib/works.bib")

--- bibliography-no-title render html ---
@distress
#bibliography("/assets/bib/works.bib", title: none)

--- bibliography-custom-title render html ---
@distress
#bibliography("/assets/bib/works.bib", title: [My References])

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

--- bibliography-multiple-files render html ---
#show: it => context { set page(width: 200pt) if target() == "paged"; it }

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

--- bibliography-chicago-fullnotes-warning ---
// Test warning for deprecated alias.
// Warning: 47-66 style "chicago-fullnotes" has been deprecated in favor of "chicago-notes"
#bibliography("/assets/bib/works.bib", style: "chicago-fullnotes", title: none)

--- bibliography-csl-display render html ---
// Test a combination of CSL `display` attributes. Most of the display
// attributes are barely used by any styles, so we have a custom style here.

#let style = ```csl
  <?xml version="1.0" encoding="utf-8"?>
  <style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0">
    <info>
      <title>Test</title>
      <id>test</id>
    </info>
    <citation collapse="citation-number">
      <layout>
        <text variable="citation-number"/>
      </layout>
    </citation>
    <bibliography>
      <layout>
        <text variable="title" font-style="italic" />
        <text variable="citation-number" display="left-margin" prefix="|" suffix="|" />
        <group display="indent">
          <text term="by" suffix=" " />
          <!-- This left-margin attribute is ignored because it is in a container. -->
          <names variable="author" display="left-margin" />
        </group>
        <group display="block" prefix="(" suffix=")">
          <text term="edition" suffix=" " text-case="capitalize-first" />
          <date variable="issued"><date-part name="year"/></date>
        </group>
      </layout>
    </bibliography>
  </style>
```

#let bib = ```bib
  @article{entry1,
    title={Title 1},
    author={Author 1},
    year={2021},
  }
```

#bibliography(
  bytes(bib.text),
  style: bytes(style.text),
  title: none,
  full: true,
)
