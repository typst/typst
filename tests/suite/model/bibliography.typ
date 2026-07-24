// Test citations and bibliographies.

--- bibliography-basic paged html pdftags ---
#set pdf(standard: "ua-1")
#show: it => context { set page(width: 200pt) if target() == "paged"; it }

= Details
See also @arrgh #cite(<distress>, supplement: [p.~22]), @arrgh[p.~4], and @distress[p.~5].
#bibliography("/assets/bib/works.bib")

--- bibliography-no-title paged html ---
@distress
#bibliography("/assets/bib/works.bib", title: none)

--- bibliography-custom-title paged html ---
@distress
#bibliography("/assets/bib/works.bib", title: [My References])

--- bibliography-before-content paged ---
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

--- bibliography-multiple-files paged html ---
#show: it => context { set page(width: 200pt) if target() == "paged"; it }

#set heading(numbering: "1.")
#show bibliography: set heading(numbering: "1.")

= Multiple Bibs
Now we have multiple bibliographies containing @glacier-melt @keshav2007read
#bibliography(("/assets/bib/works.bib", "/assets/bib/works_too.bib"))

--- bibliography-source-path paged empty ---
#show heading: none
#bibliography(path("/assets/bib/works_too.bib"))

--- bibliography-source-types paged empty ---
#let src = ```yaml
hi:
  type: Book
```

#show heading: none
#bibliography((
  "/assets/bib/works.bib",
  path("/assets/bib/works_too.bib"),
  bytes(src.text)
))

--- bibliography-duplicate-key eval ---
// Error: 15-65 duplicate bibliography keys: netwok, issue201, arrgh, quark, distress, glacier-melt, tolkien54, DBLP:books/lib/Knuth86a, sharing, restful, mcintosh_anxiety, psychology25
#bibliography(("/assets/bib/works.bib", "/assets/bib/works.bib"))

--- bibliography-ordering paged ---
#set page(width: 300pt)

@mcintosh_anxiety
@psychology25

#bibliography("/assets/bib/works.bib")

--- bibliography-full paged ---
#set page(paper: "a6", height: auto)
#bibliography("/assets/bib/works_too.bib", full: true)

--- bibliography-math paged ---
#set page(width: 200pt)

@Zee04
#bibliography("/assets/bib/works_too.bib", style: "mla")

--- bibliography-grid-par paged ---
// Ensure that a grid-based bibliography does not produce paragraphs.
#show par: highlight

@Zee04
@keshav2007read

#bibliography("/assets/bib/works_too.bib")

--- bibliography-indent-par paged ---
// Ensure that an indent-based bibliography does not produce paragraphs.
#show par: highlight

@Zee04
@keshav2007read

#bibliography("/assets/bib/works_too.bib", style: "mla")

--- bibliography-style-not-suitable paged ---
// Error: 2-62 CSL style "Alphanumeric" is not suitable for bibliographies
#bibliography("/assets/bib/works.bib", style: "alphanumeric")

--- bibliography-empty-key eval ---
#let src = ```yaml
"":
  type: Book
```
// Error: 15-30 bibliography contains entry with empty key
#bibliography(bytes(src.text))

--- issue-4618-bibliography-set-heading-level paged ---
// Test that the bibliography block's heading is set to 2 by the show rule,
// and therefore should be rendered like a level-2 heading. Notably, this
// bibliography heading should not be underlined.
#show heading.where(level: 1): it => [ #underline(it.body) ]
#show bibliography: set heading(level: 2)

= Level 1
== Level 2
@Zee04

#bibliography("/assets/bib/works_too.bib")

--- bibliography-chicago-fullnotes-warning paged empty ---
// Test warning for deprecated alias.
// Warning: 47-66 style `"chicago-fullnotes"` has been deprecated in favor of `"chicago-notes"`
#bibliography("/assets/bib/works.bib", style: "chicago-fullnotes", title: none)

--- bibliography-modern-humanities-research-association-warning paged empty ---
// Test warning for deprecated alias.
// Warning: 47-87 style `"modern-humanities-research-association"` has been deprecated in favor of `"modern-humanities-research-association-notes"`
#bibliography("/assets/bib/works.bib", style: "modern-humanities-research-association", title: none)

--- bibliography-csl-display paged html ---
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

--- bibliography-group-none html ---
#set bibliography(group: none)

@keshav2007read @netwok @arrgh
#bibliography("/assets/bib/works.bib")
#bibliography("/assets/bib/works_too.bib", style: "nlm-citation-sequence")

--- bibliography-group-auto html ---
@netwok @arrgh @keshav2007read
#bibliography("/assets/bib/works.bib")
#bibliography("/assets/bib/works_too.bib")

--- bibliography-group-str html ---
@netwok @arrgh @keshav2007read
#bibliography("/assets/bib/works.bib", group: "a")
#bibliography("/assets/bib/works_too.bib", group: "b", style: "nlm-citation-sequence")

@quark @distress @keshav2007read
#bibliography("/assets/bib/works.bib", group: "a")
#bibliography("/assets/bib/works_too.bib", group: "b", style: "nlm-citation-sequence")

--- bibliography-group-full html ---
// Test that a numbered bibliography that is printed in full without any
// citations still advances the shared numbering.
#bibliography("/assets/bib/works_too.bib", full: true)
#bibliography("/assets/bib/works.bib", full: true)

--- bibliography-group-mixed-full html ---
// Test that the shared numbering continues from a bibliography with
// `full: true` into one that only contains cited entries, and that the
// latter advances the numbering by its rendered items rather than by the
// entries in its source file.
#bibliography("/assets/bib/works_too.bib", full: true)

@netwok
#bibliography("/assets/bib/works.bib")

#bibliography("/assets/bib/works_too.bib", full: true)

--- bibliography-group-mixed-styles html ---
// Test that a bibliography with a non-numeric style does not consume
// numbers from the shared numbering.
@Zee04
#bibliography("/assets/bib/works_too.bib", style: "apa")

@netwok
#bibliography("/assets/bib/works.bib")

--- bibliography-group-citation-numbers-only html ---
// Test that a bibliography whose style displays citation numbers in
// citations, but not in the bibliography itself, still advances the
// shared numbering.
#let style = ```csl
  <?xml version="1.0" encoding="utf-8"?>
  <style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0">
    <info>
      <title>Test</title>
      <id>test</id>
    </info>
    <citation>
      <layout prefix="[" suffix="]" delimiter=", ">
        <text variable="citation-number"/>
      </layout>
    </citation>
    <bibliography>
      <layout>
        <text variable="title"/>
      </layout>
    </bibliography>
  </style>
```
#set bibliography(style: bytes(style.text))

@Zee04
#bibliography("/assets/bib/works_too.bib")

@netwok
#bibliography("/assets/bib/works.bib")

--- bibliography-group-form-year html ---
// Test that a bibliography whose citations all use a form that displays no
// citation number still advances the shared numbering.
#cite(<Zee04>, form: "year")
#bibliography("/assets/bib/works_too.bib")

@netwok
#bibliography("/assets/bib/works.bib")

--- bibliography-group-full-custom-style html ---
// Test that the shared numbering also advances for a numbered custom style
// that does not declare its citation format.
#let style = ```csl
  <?xml version="1.0" encoding="utf-8"?>
  <style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0">
    <info>
      <title>Test</title>
      <id>test</id>
    </info>
    <citation>
      <layout prefix="[" suffix="]" delimiter=", ">
        <text variable="citation-number"/>
      </layout>
    </citation>
    <bibliography second-field-align="flush">
      <layout>
        <text variable="citation-number" prefix="[" suffix="]"/>
        <text variable="title"/>
      </layout>
    </bibliography>
  </style>
```

#set bibliography(style: bytes(style.text), full: true)
#bibliography("/assets/bib/works_too.bib")
#bibliography("/assets/bib/works.bib")

--- bibliography-group-out-of-order html ---
// The numbers are ordered in the order of the bibliographies, so in the prose
// they can be out of order if using shared numbering.
@keshav2007read @netwok @arrgh
#bibliography("/assets/bib/works.bib")
#bibliography("/assets/bib/works_too.bib")

--- bibliography-group-sorted-style html ---
// Test that the shared numbering also advances for a style that uses sorting.
#set bibliography(style: "association-for-computing-machinery")
@netwok @keshav2007read
#bibliography("/assets/bib/works.bib")
#bibliography("/assets/bib/works_too.bib")
