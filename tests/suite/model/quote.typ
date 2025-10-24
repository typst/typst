// Test the quote element.

--- quote-dir-author-pos render pdftags ---
// Text direction affects author positioning
#set text(font: ("Libertinus Serif", "Noto Sans Arabic"))
And I quote: #quote(attribution: [René Descartes])[cogito, ergo sum].

#set text(lang: "ar")
#quote(attribution: [عالم])[مرحبًا]

--- quote-dir-align render pdftags ---
// Text direction affects block alignment
#set text(font: ("Libertinus Serif", "Noto Sans Arabic"))
#set quote(block: true)
#quote(attribution: [René Descartes])[cogito, ergo sum]

#set text(lang: "ar")
#quote(attribution: [عالم])[مرحبًا]

--- quote-block-spacing render ---
// Spacing with other blocks
#set quote(block: true)
#set text(8pt)

#lines(3)
#quote(lines(3))
#lines(3)

--- quote-inline render ---
// Inline citation
#set text(8pt)
#quote(attribution: <tolkien54>)[In a hole in the ground there lived a hobbit.]

#show bibliography: none
#bibliography("/assets/bib/works.bib")

--- quote-cite-format-label-or-numeric render ---
// Citation-format: label or numeric
#set text(8pt)
#set quote(block: true)
#quote(attribution: <tolkien54>)[In a hole in the ground there lived a hobbit.]

#show bibliography: none
#bibliography("/assets/bib/works.bib", style: "ieee")

--- quote-cite-format-note render ---
// Citation-format: note
#set text(8pt)
#set quote(block: true)
#quote(attribution: <tolkien54>)[In a hole in the ground there lived a hobbit.]

#show bibliography: none
#bibliography("/assets/bib/works.bib", style: "chicago-shortened-notes")

--- quote-cite-format-author-date render ---
// Citation-format: author-date or author
#set text(8pt)
#set quote(block: true)
#quote(attribution: <tolkien54>)[In a hole in the ground there lived a hobbit.]

#show bibliography: none
#bibliography("/assets/bib/works.bib", style: "apa")

--- quote-nesting render ---
// Test quote selection.
#set page(width: auto)
#set text(lang: "en")
=== EN
#quote[An apostroph'] \
#quote[A #quote[nested] quote] \
#quote[A #quote[very #quote[nested]] quote]

#set text(lang: "de")
=== DE
#quote[Satz mit Apostroph'] \
#quote[Satz mit #quote[Zitat]] \
#quote[A #quote[very #quote[nested]] quote]

#set smartquote(alternative: true)
=== DE Alternative
#quote[Satz mit Apostroph'] \
#quote[Satz mit #quote[Zitat]] \
#quote[A #quote[very #quote[nested]] quote]

--- quote-nesting-custom render ---
// With custom quotes.
#set smartquote(quotes: (single: ("<", ">"), double: ("(", ")")))
#quote[A #quote[nested] quote]

--- quote-plato html ---
#set quote(block: true)

#quote(attribution: [Plato])[
  ... ἔοικα γοῦν τούτου γε σμικρῷ τινι αὐτῷ τούτῳ σοφώτερος εἶναι, ὅτι
  ἃ μὴ οἶδα οὐδὲ οἴομαι εἰδέναι.
]
#quote(attribution: [from the Henry Cary literal translation of 1897])[
  ... I seem, then, in just this little thing to be wiser than this man at
  any rate, that what I do not know I do not think I know either.
]

--- quote-nesting-html html ---
When you said that #quote[he surely meant that #quote[she intended to say #quote[I'm sorry]]], I was quite confused.

--- quote-attribution-link html ---
#quote(
  block: true,
  attribution: link("https://typst.app/home")[typst.com]
)[
  Compose papers faster
]

--- quote-par render ---
// Ensure that an inline quote is part of a paragraph, but a block quote
// does not result in paragraphs.
#show par: highlight

An inline #quote[quote.]

#quote(block: true, attribution: [The Test Author])[
  A block-level quote.
]

--- issue-5536-quote-inline-quotes-false render ---
Lorem #quote(block: false, quotes: false)[dolor].
