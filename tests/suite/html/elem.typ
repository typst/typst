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

--- html-elem-custom html ---
#html.elem("my-element")[Hi]
#html.elem("custom-button")[Hi]
#html.elem("multi-word-component")[Hi]
#html.elem("element-")[Hi]

--- html-elem-invalid html ---
// Error: 12-24 the character "@" is not valid in a tag name
#html.elem("my@element")

--- html-elem-custom-bad-start html ---
// Error: 12-22 custom element name must start with a lowercase letter
#html.elem("1-custom")

--- html-elem-custom-uppercase html ---
// Error: 12-21 custom element name must not contain uppercase letters
#html.elem("my-ELEM")

--- html-elem-custom-reserved html ---
// Error: 12-28 name is reserved and not valid for a custom element
#html.elem("annotation-xml")
