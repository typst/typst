--- html-elem-alone-context html ---
#context html.elem("html")

--- html-elem-not-alone html ---
// Error: 2-19 `<html>` element must be the only element in the document
#html.elem("html")
Text

--- html-elem-metadata html ---
#html.elem("html", context {
  let val = query(<l>).first().value
  test(val, "Hi")
  val
})
#metadata("Hi") <l>

--- issue-5907-html-elem-at-root html ---
#html.elem("span", [Not wrapped in p tag])

Wrapped in p tag
