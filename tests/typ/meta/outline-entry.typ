// Tests outline entry.

---
#set page(width: 150pt)
#set heading(numbering: "a.")
#show outline.entry.where(level: 1): strong
#show outline.entry.where(level: 2): set text(red)
#show outline.entry.where(level: 4): it => {
    if it.fill != repeat[-] {
        outline.entry(
            level: 4,
            element: it.element,
            outline: {
                set text(red)
                it.outline
            },
            fill: repeat[-]
        )
    } else {
        it
    }
}

#outline()

= Top heading

#lorem(10)

== Not top heading

#lorem(10)

=== Lower heading

#lorem(10)

=== Lower too

#lorem(10)

== Also not top

#lorem(10)

= Another top heading

== Middle heading

=== Lower heading

==== Lowest heading

---
#set page(width: 150pt, numbering: "I")
#set heading(numbering: "1.")
#show outline.entry.where(level: 1): it => locate(loc => {
    let elem-loc = it.element.location()
    let page-numbering = elem-loc.page-numbering()
    let page-number = numbering(page-numbering, elem-loc.page())
    [
        #set text(blue)
        #emph(link(elem-loc, it.outline))
        #box(width: 1fr, repeat(box[O#it.fill.body]))
        #[
            #set text(red)
            #link(elem-loc, page-number)
        ]
    ]
})

#outline(indent: auto, fill: repeat[!])

= Top heading

#lorem(10)

== Not top heading

#lorem(10)

=== Lower heading

#lorem(10)

=== Lower too

#lorem(10)

== Also not top

#lorem(10)

= Another top heading

== Middle heading

=== Lower heading

#lorem(10)

---
// Error: 2-23 cannot outline cite
#outline(target: cite)
#cite("arrgh", "distress", [p. 22])
#bibliography("/works.bib")
