--- cite-footnote ---
Hello @netwok
And again: @netwok

#pagebreak()
#bibliography("/assets/bib/works.bib", style: "chicago-shortened-notes")

--- cite-form ---
#set page(width: 200pt)

Nothing: #cite(<arrgh>, form: none)

#cite(<netwok>, form: "prose") say stuff.

#bibliography("/assets/bib/works.bib", style: "apa")

--- cite-group ---
A#[@netwok@arrgh]B \
A@netwok@arrgh B \
A@netwok @arrgh B \
A@netwok @arrgh. B \

A @netwok#[@arrgh]B \
A @netwok@arrgh, B \
A @netwok @arrgh, B \
A @netwok @arrgh. B \

A#[@netwok @arrgh @quark]B. \
A @netwok @arrgh @quark B. \
A @netwok @arrgh @quark, B.

#set text(0pt)
#bibliography("/assets/bib/works.bib", style: "american-physics-society")

--- cite-grouping-and-ordering ---
@mcintosh_anxiety
@psychology25
@netwok
@issue201
@arrgh
@quark
@distress,
@glacier-melt
@issue201
@tolkien54
@sharing
@restful

#show bibliography: none
#bibliography("/assets/bib/works.bib", style: "american-physics-society")

--- issue-785-cite-locate ---
// Test citation in other introspection.
#set page(width: 180pt)
#set heading(numbering: "1.")

#outline(
  title: [Figures],
  target: figure.where(kind: image),
)

#pagebreak()

= Introduction <intro>
#figure(
  rect(height: 10pt),
  caption: [A pirate @arrgh in @intro],
)

#context [Citation @distress on page #here().page()]

#show bibliography: none
#bibliography("/assets/bib/works.bib", style: "chicago-shortened-notes")

--- issue-1597-cite-footnote ---
// Tests that when a citation footnote is pushed to next page, things still
// work as expected.
#set page(height: 60pt)
A

#footnote[@netwok]
#show bibliography: none
#bibliography("/assets/bib/works.bib")

--- issue-2531-cite-show-set ---
// Test show set rules on citations.
#show cite: set text(red)
A @netwok @arrgh.
B #cite(<netwok>) #cite(<arrgh>).

#show bibliography: none
#bibliography("/assets/bib/works.bib")

--- issue-3481-cite-location ---
// The locator was cloned in the wrong location, leading to inconsistent
// citation group locations in the second footnote attempt.
#set page(height: 60pt)

// First page shouldn't be empty because otherwise we won't skip the first
// region which causes the bug in the first place.
#v(10pt)

// Everything moves to the second page because we want to keep the line and
// its footnotes together.
#footnote[@netwok \ A]

#show bibliography: none
#bibliography("/assets/bib/works.bib")

--- issue-3699-cite-twice-et-al ---
// Citing a second time showed all authors instead of "et al".
@mcintosh_anxiety \
@mcintosh_anxiety
#show bibliography: none
#bibliography("/assets/bib/works.bib", style: "chicago-author-date")

--- issue-5503-cite-in-align ---
// The two aligned elements should be displayed in separate lines.
#align(right)[@netwok]
#align(right)[b]

#show bibliography: none
#bibliography("/assets/bib/works.bib")

--- issue-5503-cite-group-interrupted-by-par-align ---
// `par` and `align` are block-level and should interrupt a cite group
@netwok
@arrgh
#par(leading: 5em)[@netwok]
#par[@arrgh]
@netwok
@arrgh
#align(right)[@netwok]
@arrgh

#show bibliography: none
#bibliography("/assets/bib/works.bib")

--- cite-type-error-hint ---
// Test hint for cast error from str to label
// Error: 7-15 expected label, found string
// Hint: 7-15 use `<netwok>` or `label("netwok")` to create a label
#cite("netwok")

--- cite-type-error-hint-invalid-literal ---
// Test hint for cast error from str to label
// Error: 7-17 expected label, found string
// Hint: 7-17 use `label("%@&#*!\\")` to create a label
#cite("%@&#*!\\")

--- issue-5775-cite-order-rtl ---
// Test citation order in RTL text.
#set page(width: 300pt)
#set text(font: ("Libertinus Serif", "Noto Sans Arabic"))
@netwok
aaa
این است
@tolkien54
و این یکی هست
@arrgh

#bibliography("/assets/bib/works.bib")

--- cite-chicago-fullnotes-warning ---
// Test warning for deprecated alias.
// Warning: 24-43 style "chicago-fullnotes" has been deprecated in favor of "chicago-notes"
#cite(<netwok>, style: "chicago-fullnotes")
#bibliography("/assets/bib/works.bib")

--- cite-chicago-fullnotes-set-rule-warning ---
// Test warning for deprecated alias.
// Warning: 18-37 style "chicago-fullnotes" has been deprecated in favor of "chicago-notes"
#set cite(style: "chicago-fullnotes")

--- cite-supplements-and-ibid ---
#set page(width: 300pt)

Par 1 @arrgh

Par 2 @arrgh[p. 5-8]

Par 3 @arrgh[p. 5-8]

Par 4 @arrgh[p. 9-10]

#let style = bytes(
  ```xml
  <?xml version=\"1.0\" encoding=\"utf-8\"?>"
  <style xmlns=\"http://purl.org/net/xbiblio/csl\" version=\"1.0\" class=\"note\" default-locale=\"pl-PL\">
    <info>
      <title>Example citation style</title>
      <id>http://www.example.com/</id>
    </info>
    <macro name=\"locator\">
      <group delimiter=\" \">
        <label variable=\"locator\" form=\"short\"/>
        <text variable=\"locator\"/>
      </group>
    </macro>
  
    <citation>
      <sort>
        <key variable=\"title\"/>
        <key variable=\"issued\"/>
      </sort>
      <layout>
        <choose>
          <if position=\"first\">
            <group delimiter=\", \">
              <text variable=\"title\"/>
              <text macro=\"locator\"/>
            </group>
          </if>
          <else-if position=\"ibid-with-locator\">
            <group delimiter=\", \">
              <text term=\"ibid\"/>
              <text macro=\"locator\"/>
            </group>
          </else-if>
          <else-if position=\"ibid\">
            <text term=\"ibid\"/>
          </else-if>
          <else-if position=\"subsequent\">
            <group delimiter=\", \">
              <text variable=\"title\"/>
              <text macro=\"locator\"/>
            </group>
          </else-if>
        </choose>
      </layout>
    </citation>
    <bibliography>
      <sort>
        <key variable=\"title\"/>
      </sort>
      <layout>
        <text variable=\"title\"/>
      </layout>
    </bibliography>
  </style>```.text
)

#bibliography("/assets/bib/works.bib", style: style)
