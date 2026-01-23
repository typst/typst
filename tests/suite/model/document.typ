// Test document and page-level styles.

--- document-set-title paged ---
#set document(title: [Hello])
What's up?

--- document-set-author-date paged empty ---
#set document(author: ("A", "B"), date: datetime.today())

--- document-date-bad eval ---
// Error: 21-28 expected datetime, none, or auto, found string
#set document(date: "today")

--- document-author-bad eval ---
// Error: 23-29 expected string, found integer
#set document(author: (123,))
What's up?

--- document-set-after-content paged ---
// Document set rules can appear anywhere in top-level realization, also after
// content.
Hello
#set document(title: [Hello])

--- document-constructor bundle ---
#document("a.pdf")[This is a PDF document]

--- document-constructor-incomplete eval ---
// Error: 2-12 missing argument: path
#document()

--- document-path-is-root-1 eval ---
// Error: 11-14 path must have at least one component
#document("/", format: "pdf")[Root?]

--- document-path-is-root-2 eval ---
// Error: 11-15 path must have at least one component
#document("/.", format: "pdf")[Root?]

--- document-path-escapes eval ---
// Error: 11-15 path `".."` would escape the bundle root
#document("..", format: "pdf")[Root?]

--- document-weird-extension bundle ---
// The hashes of both files should match.
#document("normal.pdf")[A PDF]
#document("weird.oops", format: "pdf")[A PDF]

--- document-unknown-format-inferred bundle ---
// Error: 2-21 unknown document format
// Hint: 2-21 try specifying the `format` explicitly
#document("a.txt")[]

--- document-unknown-format-specified eval ---
// Error: 28-33 expected "pdf", "png", "svg", "html", or auto
#document("a.pdf", format: "txt")[]

--- document-path-collision bundle ---
// Hint: 2-26 path is already in use here
#document("a.pdf")[Doc A]
// Hint: 2-26 path is already in use here
// Hint: 2-26 path is already in use here
#document("b.pdf")[Doc A]
// Error: 2-27 path `a.pdf` occurs multiple times in the bundle
// Hint: 2-27 document paths must be unique in the bundle
#document("a.pdf")[Doc A']
// Error: 2-27 path `b.pdf` occurs multiple times in the bundle
// Hint: 2-27 document paths must be unique in the bundle
#document("b.pdf")[Doc B']
// Error: 2-28 path `b.pdf` occurs multiple times in the bundle
// Hint: 2-28 asset paths must be unique in the bundle
#asset("b.pdf", "Fake PDF")
#document("c.pdf")[Doc C]

--- document-outside-of-bundle paged ---
// Error: 2-21 constructing a document is only supported in the bundle target
// Hint: 2-21 try enabling the bundle target
// Hint: 2-21 or use a `set document(..)` rule to configure metadata
#document("a.pdf")[]

--- document-format-outside-of-bundle paged html ---
// Error: 2-30 setting the document format is only supported in the bundle target
#set document(format: "html")

--- document-format-in-container paged html ---
#block[
  // Error: 4-32 document set rules are not allowed inside of containers
  #set document(format: "html")
]

--- document-nested bundle ---
// The error here is not ideal ...
#document("a.pdf")[
  // Error: 4-23 constructing a document is only supported in the bundle target
  // Hint: 4-23 try enabling the bundle target
  // Hint: 4-23 or use a `set document(..)` rule to configure metadata
  #document("c.pdf")[]
]

--- document-image-multi-page bundle ---
#let multi = [
  #set page(height: 100pt)
  #rect(height: 100pt)
  #rect(height: 100pt)
]

// Error: 2-30 expected document to have a single page
// Hint: 2-30 the document resulted in 2 pages
// Hint: 2-30 documents exported to an image format only support a single page
#document("image.svg", multi)

// Error: 2-30 expected document to have a single page
// Hint: 2-30 the document resulted in 2 pages
// Hint: 2-30 documents exported to an image format only support a single page
#document("image.png", multi)

--- document-realization-errors bundle ---
// This test ensures that we show errors from all document realizations at once.

// Error: 30-37 cannot add integer and string
#document("bar.pdf", context 1 + "2")
// Error: 23-27 label `<bar>` does not exist in the document
#document("foo.pdf", [@bar])

--- document-export-errors bundle ---
// This test ensures that we show errors from all exports at once when trying to
// create a bundle.

// This document is fine.
#document(
  "foo.html",
  [Hello],
)

// This one errors.
#document(
  "bar.html",
  // Error: 3-27 HTML raw text element cannot contain its own closing tag
  // Hint: 3-27 the sequence `</script` appears in the raw text
  html.script("</script>"),
)

// This one errors, too.
#document(
  "baz.html",
  // Error: 3-25 HTML raw text element cannot contain its own closing tag
  // Hint: 3-25 the sequence `</style` appears in the raw text
  html.style("</style>"),
)

--- document-content-without-document bundle ---
// Error: 1-8 heading is not allowed at the top-level in bundle export
// Hint: 1-8 try wrapping the content in a `document` instead
= Hello
// Error: 1-6 text is not allowed at the top-level in bundle export
// Hint: 1-6 try wrapping the content in a `document` instead
world

--- document-properties-in-bundle bundle ---
// Ensures that the title element works correctly in bundle export.
// See the comment in `impl ShowSet for Packed<DocumentElem>` for more details.
#document(
  "hi.html",
  title: [Test title],
  author: "Test Author",
  description: [Test description],
  keywords: ("a", "b"),
)[
  #title()
  // Testing all properties separately because, in the current implementation,
  // they are manually "forwarded".
  #context {
    test(document.format, auto)
    test(document.title, [Test title])
    test(document.author, ("Test Author",))
    test(document.description, [Test description])
    test(document.keywords, ("a", "b"))
    test(document.date, auto)
  }
]

--- document-properties-precedence bundle ---
// Tests the precedence of different ways to specify document metadata. Each
// subcase's filename specifies the expected metadata output (for HTML, in the
// tab bar) and printed output (what `#title()` displays), in that order.

// Just an arg of course works.
#document("1-arg-arg.html", title: [Arg], title())

// Just a set rule, too.
#for ext in ("html", "pdf") {
  set document(title: [Outer])
  document("2-outer-outer." + ext, title())
}

// An explicit arg wins against a set rule, as usual.
#{
  set document(title: [Outer])
  document("3-arg-arg.html", title: [Arg], title())
}

// An interior set rule is also supported. While that's not the usual behavior
// of Typst, this is how document metadata is configured in single-document
// export. Supporting it in bundles, too, makes it easier to compose bundle
// export with documents that also work standalone, e.g. something like
// `document("paper.pdf", include "paper.typ")`.
#document("4-inner-inner.html", {
  set document(title: [Inner])
  title()
})

// An interior set rule wins against an exterior one. The outer set rule can be
// viewed similarly to the global default in its effect, so given that we want
// to support interior `set document` at all, the inner one should win here.
// Otherwise, the inner one wouldn't win against the global default either,
// which makes no sense.
#{
  set document(title: [Outer])
  document("5-inner-inner.html", {
    set document(title: [Inner])
    title()
  })
}

// An interior set rule wins even against an explicit argument. This is a
// little odd, but letting `document(title: ..)` have precedence is also
// problematic, for two reasons:
//
// - By the time we're compiling the document, we can't distinguish whether
//   `[Arg]` is an inherent property, came from a set rule, or is just the
//   global default because the document element is already materialized. So if
//   we always used that materialized property, an inner `set document` wouldn't
//   ever have a chance to work. We could accept that, but having the title work
//   in something like `document("paper.pdf", include "paper.typ")` where the
//   paper already has a document set rule is too useful to ignore.
//
// - Even if we could make `[Arg]` win here, it would in turn be somewhat odd if
//   moving bundle-level explicit arguments to a bundle-level set rule would not
//   work (because `[Inner]` would definitely win against `[Outer]` if we want
//   to support inner set document rules at all; see the `5-inner-inner.html`
//   case).
//
// With the chosen implementation, all of bundle-level set rules, explicit
// document constructor arguments, and set document rules within the document
// work cleanly if used in isolation. The only slighty quirky behavior is that
// an inner `set document` wins against an explicit argument.
#for ext in ("html", "pdf") {
  set document(title: [Outer])
  document("6-inner-inner." + ext, title: [Arg])[
    #set document(title: [Inner])
    #title()
  ]
}

--- document-property-bad-in-bundle bundle ---
// This does not work because it's a required argument and not a settable one.
// Error: 39-43 function `document` does not contain field `path`
#document("hi.html", context document.path)

--- document-set-in-container paged ---
#box[
  // Error: 4-32 document set rules are not allowed inside of containers
  #set document(title: [Hello])
]

--- issue-4065-document-context paged empty ---
// Test that we can set document properties based on context.
#show: body => context {
  let all = query(heading)
  let title = if all.len() > 0 { all.first().body }
  set document(title: title)
  body
}

#show heading: none
= Top level

--- issue-4769-document-context-conditional paged empty ---
// Test that document set rule can be conditional on document information
// itself.
#set document(author: "Normal", title: [Alternative])
#context {
  set document(author: "Changed") if "Normal" in document.author
}
