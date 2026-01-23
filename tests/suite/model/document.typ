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
