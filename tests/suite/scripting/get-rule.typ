--- get-rule-basic paged ---
// Test basic get rule.
#context test(text.lang, "en")
#set text(lang: "de")
#context test(text.lang, "de")
#text(lang: "es", context test(text.lang, "es"))

--- get-rule-in-function paged ---
// Test whether context is retained in nested function.
#let translate(..args) = args.named().at(text.lang)
#set text(lang: "de")
#context test(translate(de: "Inhalt", en: "Contents"), "Inhalt")

--- get-rule-in-array-callback paged ---
// Test whether context is retained in built-in callback.
#set text(lang: "de")
#context test(
  ("en", "de", "fr").sorted(key: v => v != text.lang),
  ("de", "en", "fr"),
)

--- get-rule-folding paged ---
// Test folding.
#set rect(stroke: red)
#context {
  test(type(rect.stroke), stroke)
  test(rect.stroke.paint, red)
}
#[
  #set rect(stroke: 4pt)
  #context test(rect.stroke, 4pt + red)
]
#context test(rect.stroke, stroke(red))

--- get-rule-figure-caption-collision paged ---
// We have one collision: `figure.caption` could be both the element and a get
// rule for the `caption` field, which is settable. We always prefer the
// element. It's unfortunate, but probably nobody writes
// `set figure(caption: ..)` anyway.
#test(type(figure.caption), function)
#context test(type(figure.caption), function)

--- get-rule-assertion-failure paged ---
// Error: 10-31 Assertion failed: "en" != "de"
#context test(text.lang, "de")

--- get-rule-unknown-field paged ---
// Error: 15-20 function `text` does not contain field `langs`
#context text.langs

--- get-rule-inherent-field paged ---
// Error: 18-22 function `heading` does not contain field `body`
#context heading.body

--- get-rule-missing-context-no-context paged ---
// Error: 7-11 can only be used when context is known
// Hint: 7-11 try wrapping this in a `context` expression
// Hint: 7-11 the `context` expression should wrap everything that depends on this function
#text.lang

--- get-rule-unknown-field-no-context paged ---
// Error: 7-12 function `text` does not contain field `langs`
#text.langs

--- get-rule-inherent-field-no-context paged ---
// Error: 10-14 function `heading` does not contain field `body`
#heading.body
