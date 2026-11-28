// Test interactive forms.

--- form-basic-pdf-ua-1 paged html pdftags ---
#set pdf(standard: "ua-1")

// One of each field type.

#form[
  #form.button(action: "reset")[Reset button]

  #form.label(<checkbox>)[Checkbox]
  #form.checkbox() <checkbox>

  #form.radio-group(title: [A radio group])[
    #form.label(<radio>)[Radio]
    #form.radio() <radio>
  ]

  #form.label(<textbox>)[Textbox]
  #form.textbox(width: 100pt) <textbox>

  #form.label(<textarea>)[Textarea]
  #form.textbox(kind: "multi-line") <textarea>

  #form.label(<dropdown>)[Dropdown]
  #form.choice(options: ("foo", "bar")) <dropdown>

  #form.label(<listbox>)[Listbox]
  #form.choice(options: ("foo", "bar"), multiple: true) <listbox>
]

--- form-basic-pdf-a-3a paged html pdftags ---
#set pdf(standard: "a-3a")
#set document(date: datetime(year: 1970, month: 1, day: 1))

// One of each field type.

#form[
  #form.button()[Button]

  #form.label(<checkbox>)[Checkbox]
  #form.checkbox() <checkbox>

  #form.radio-group(title: [A radio group])[
    #form.label(<radio>)[Radio]
    #form.radio() <radio>
  ]

  #form.label(<textbox>)[Textbox]
  #form.textbox(width: 100pt) <textbox>

  #form.label(<textarea>)[Textarea]
  #form.textbox(kind: "multi-line") <textarea>

  #form.label(<dropdown>)[Dropdown]
  #form.choice(options: ("foo", "bar")) <dropdown>

  #form.label(<listbox>)[Listbox]
  #form.choice(options: ("foo", "bar"), multiple: true) <listbox>
]

--- form-signature paged ---
#form[
  #form.label(<sig>)[Sign here]
  #form.signature() <sig>
]

--- form-tagged-in-label paged pdftags ---
#form[
  #form.label(<cb>)[
    *Lorem Ipsum*

    - Item 1
    - _Item 2_
  ]

  #form.checkbox() <cb>
]

--- form-tagged-in-button paged pdftags ---
#form[
  #form.button[
    *Lorem Ipsum*

    - Item 1
    - _Item 2_
  ]
]

--- form-link-in-button pdf ---
#form[
  // Error: 1:4-3:4 form fields may not contain links
  // Hint: 1:4-3:4 a link was used within this form field
  // Hint: 1:4-3:4 references, citations, and footnotes are also considered links in PDF
  #form.button[
    #link("https://example.com")[link]
  ]
]

--- form-target-invalid-content-type-html html ---
// HTML cannot export to FDF, XFDF, nor PDF.
// Error: 15-64 content-type `"fdf"` is not supported in HTML export
#form(target: (url: "https://example.com", content-type: "fdf"))[]
// Error: 15-65 content-type `"xfdf"` is not supported in HTML export
#form(target: (url: "https://example.com", content-type: "xfdf"))[]
// Error: 15-64 content-type `"pdf"` is not supported in HTML export
#form(target: (url: "https://example.com", content-type: "pdf"))[]

--- form-target-invalid-content-type-pdf pdf ---
// PDF cannot export to multipart nor plain text.
// Error is only triggered when there is a submit button.

// Error: 15-70 content-type `"multipart"` is not supported in PDF export
#form(target: (url: "https://example.com", content-type: "multipart"))[
  #form.button(action: "submit")[]
]

// Error: 15-66 content-type `"plain"` is not supported in PDF export
#form(target: (url: "https://example.com", content-type: "plain"))[
  #form.button(action: "submit")[]
]

--- form-target-warning-content-type-pdf pdf ---
// PDF readers have poor support for all formats that are not FDF.
// Warning is only triggered when there is a submit button.

// Warning: 15-72 content-type `"url-encoded"` has poor support among PDF readers
// Hint: 15-72 prefer the `"fdf"` content-type whenever possible
#form(target: (url: "https://example.com", content-type: "url-encoded"))[
  #form.button(action: "submit")[Submit]
]

// Warning: 15-65 content-type `"xfdf"` has poor support among PDF readers
// Hint: 15-65 prefer the `"fdf"` content-type whenever possible
#form(target: (url: "https://example.com", content-type: "xfdf"))[
  #form.button(action: "submit")[Submit]
]

// Warning: 15-64 content-type `"pdf"` has poor support among PDF readers
// Hint: 15-64 prefer the `"fdf"` content-type whenever possible
#form(target: (url: "https://example.com", content-type: "pdf"))[
  #form.button(action: "submit")[Submit]
]

--- form-target-method-get-pdf pdf ---
// PDF cannot use GET method.
// Error is only triggered when there is a submit button.
// Error: 15-58 method `"get"` is not supported in PDF export
#form(target: (url: "https://example.com", method: "get"))[
  #form.button(action: "submit")[]
]

--- form-target-method-get-html html ---
// HTML can only use GET method on auto/url-encoded.
#form(target: (url: "https://example.com", method: "get"))[]
#form(target: (url: "https://example.com", content-type: "url-encoded", method: "get"))[]
// Error: 15-85 method `"get"` is only supported with content-type `"url-encoded"`
#form(target: (url: "https://example.com", content-type: "multipart", method: "get"))[]
// Error: 15-81 method `"get"` is only supported with content-type `"url-encoded"`
#form(target: (url: "https://example.com", content-type: "plain", method: "get"))[]

--- form-label-across-documents bundle ---
// HTML labels can only target fields in the same document.
#document("index.html")[
  // Error: 4-38 form labels can only target fields in the same document
  #form.label(<checkbox>)[A checkbox]
]
#document("page.html")[
  #form[
    #form.checkbox() <checkbox>
  ]
]

--- form-label-not-field pdf html ---
#form[
  = Heading <heading>

  // Error: 4-32 label `<heading>` does not point to a form field
  #form.label(<heading>)[Label]
]

--- form-submit-button-without-target paged ---
// Hint: 1:2-3:2 set the `target` property of the surrounding `#form` element
#form[
  // Error: 4-41 button has a submit action, but no submission target is defined for its form
  #form.button(action: "submit")[Submit]
]

--- form-radio-group-selected-not-button pdf html ---
#form[
  == Heading <heading>

  // Error: 1:4-4:4 label `<heading>` does not exist in the radio group
  #form.radio-group(selected: <heading>)[
    #form.radio() <radio-1>
    #form.radio() <radio-2>
  ]
]

--- form-radio-group-selected-multiple pdf html ---
#form[
  // Error: 1:4-4:4 label `<radio-1>` occurs multiple times in the radio group
  #form.radio-group(selected: <radio-1>)[
    #form.radio() <radio-1>
    #form.radio() <radio-1>
  ]
]

--- form-radio-group-selected-outside-group pdf html ---
#form[
  #form.radio-group()[
    #form.radio() <radio-1>
    #form.radio() <radio-2>
  ]

  // Error: 1:4-4:4 label `<radio-2>` does not exist in the radio group
  #form.radio-group(selected: <radio-2>)[
    #form.radio() <radio-3>
    #form.radio() <radio-4>
  ]
]

--- form-choice-duplicate-options paged html ---
#form[
  // Error: 1:4-3:4 duplicate option `"foo"`
  // Hint: 1:4-3:4 occurred in options at indices 0 and 2
  #form.choice(
    options: ("foo", "bar", "foo", "baz"),
  )
]

--- form-choice-multiple-values-on-dropdown paged html ---
#form[
  // Error: 1:4-4:4 field cannot have more than one value when `multiple` is `false`
  // Hint: 1:4-4:4 set `multiple` to `true` or set only a single value for this field
  #form.choice(
    options: ("foo", "bar", "baz"),
    value: ("bar", "baz"),
  )
]

--- form-choice-unknown-value paged html ---
#form[
  // Error: 1:4-4:4 unknown value `"Foo"`
  // Hint: 1:4-4:4 value is not one of the options of this field
  // Hint: 1:4-4:4 try adding it to the `options` property or setting a different value instead
  #form.choice(
    options: ("foo": "Foo"),
    value: "Foo",
  )
]

--- form-widget-cannot-be-contructed eval ---
// Error: 2-15 cannot be constructed manually
#form.widget()

--- form-pdf-2_0-standard-fonts pdf ---
// FIXME: this test should stop throwing an error whenever this is fixed.
// Error: a PDF standard font was used, but that is not allowed in PDF 2.0
// Hint: these fonts are used by textboxes and choice fields
// Hint: set the PDF version to PDF 1.7 or earlier, or remove form fields
#set pdf(standard: "2.0")

#form[
  #form.textbox()
]

--- form-variable-text-non-solid-color pdf ---
#let color = gradient.linear(red, green)

#form[
  #show form.textbox: set text(fill: color)
  // Error: 4-18 text of a form field can only be filled with a solid color
  // Hint: 4-18 remove the gradient or pattern
  #form.textbox()

  #show form.choice: set text(fill: color)
  // Error: 4-17 text of a form field can only be filled with a solid color
  // Hint: 4-17 remove the gradient or pattern
  #form.choice()
  // Error: 4-31 text of a form field can only be filled with a solid color
  // Hint: 4-31 remove the gradient or pattern
  #form.choice(multiple: true)
]

--- form-duplicate-field-names pdf ---
#form[
  // Hint: 4-30 the other field is here
  #form.checkbox(name: "foo")
  // Error: 4-30 two distinct form fields are named `"foo"`
  // Hint: 4-30 change the name of one of the fields
  #form.checkbox(name: "foo")
]

--- form-duplicate-fully-qualified-field-names pdf ---
#form[
  // Error: 4-34 form field name `"foo.bar"` conflicts with `"foo.bar.baz"`
  // Hint: 4-34 change the names so that `"foo.bar"` isn't fully contained by `"foo.bar.baz"`
  // Hint: 4-34 form fields use dot-separated paths to define nested fields
  #form.checkbox(name: "foo.bar")
  // Hint: 4-38 the field `"foo.bar.baz"` is here
  #form.checkbox(name: "foo.bar.baz")
]

--- form-duplicate-fully-qualified-field-names-inverse pdf ---
#form[
  // Hint: 4-38 the field `"foo.bar.baz"` is here
  #form.checkbox(name: "foo.bar.baz")
  // Error: 4-34 form field name `"foo.bar"` conflicts with `"foo.bar.baz"`
  // Hint: 4-34 change the names so that `"foo.bar"` isn't fully contained by `"foo.bar.baz"`
  // Hint: 4-34 form fields use dot-separated paths to define nested fields
  #form.checkbox(name: "foo.bar")
]

--- form-field-outside-form paged html ---
// Error: 2-15 this button element must be placed inside a form element
// Hint: 2-15 try surrounding this element with `#form[...]`
#form.button[]
// Error: 2-17 this checkbox element must be placed inside a form element
// Hint: 2-17 try surrounding this element with `#form[...]`
#form.checkbox()
// Error: 2-20 this radio-group element must be placed inside a form element
// Hint: 2-20 try surrounding this element with `#form[...]`
#form.radio-group[]
// Error: 2-16 this textbox element must be placed inside a form element
// Hint: 2-16 try surrounding this element with `#form[...]`
#form.textbox()
// Error: 2-15 this choice element must be placed inside a form element
// Hint: 2-15 try surrounding this element with `#form[...]`
#form.choice()

--- form-radio-button-outside-radio-group paged html ---
#form[
  // Error: 4-16 this radio button must be placed inside a radio group
  // Hint: 4-16 try surrounding this button with `#form.radio-group[...]`
  #form.radio()
]

--- form-missing-field-alt-name-pdf-ua pdf ---
#set pdf(standard: "ua-1")

#form[
  // Error: 4-18 PDF/UA-1 error: missing alt name
  // Hint: 4-18 add a `#form.label` that points to this field
  // Hint: 4-18 for `#form.radio-group`, you must set the `title` property instead
  #form.textbox()
]

--- form-hover-down-styles-pdf-a pdf ---
// Error: PDF/A-3a error: missing document date
// Hint: set the date of the document
#set pdf(standard: "a-3a")

#form[
  #show form.checkbox: it => {
    show form.widget.where(state: "hovered"): set rect(stroke: red)
    it
  }
  #form.label(<checkbox-1>)[Checkbox]
  // Error: 4-19 PDF/A-3a error: annotation has hover or down styles
  // Hint: 4-19 hover and down styles are not supported in this PDF standard
  // Hint: 4-19 remove `form.widget` show rules that vary on the widget's `state`
  #form.checkbox() <checkbox-1>
]

#form[
  #show form.checkbox: it => {
    show form.widget.where(state: "down"): set rect(stroke: yellow)
    it
  }
  #form.label(<checkbox-2>)[Checkbox]
  // Error: 4-19 PDF/A-3a error: annotation has hover or down styles
  // Hint: 4-19 hover and down styles are not supported in this PDF standard
  // Hint: 4-19 remove `form.widget` show rules that vary on the widget's `state`
  #form.checkbox() <checkbox-2>
]

--- form-textbox-fixed-width-errors paged ---
#form[
  // Error: 4-37 `max-length` must be set when `kind` is `"fixed-width"`
  #form.textbox(kind: "fixed-width")
]

--- form-textbox-fixed-width-paged paged ---
#form[
  #set form.textbox(kind: "fixed-width", max-length: 6, width: 100pt)
  #form.textbox()
  #form.textbox(value: "abcdef")
  #form.textbox(value: "分别设置")
  #form.textbox(value: "عليك")
]

--- form-textbox-fixed-width-no-separators paged ---
// Typst will not draw separators if rect has non-uniform stroke.
#form[
  #show form.textbox: set rect(stroke: (x: red, y: blue))
  #form.textbox(kind: "fixed-width", value: "abc", max-length: 6, width: 100pt)
]

--- form-textbox-fixed-width-html html ---
#form[
  // Warning: 4-37 `kind: "fixed-width"` property was ignored during HTML export
  // Hint: 4-37 fixed-width textboxes are not supported in this export format
  // Hint: 4-37 textbox is treated as single-line instead
  #form.textbox(kind: "fixed-width")
]

--- form-textbox-max-length paged html ---
#form[
  #set form.textbox(width: 100pt)
  #form.textbox(max-length: 10)
  #form.textbox(kind: "multi-line", max-length: 100)
]

--- form-textbox-max-length-errors paged html ---
#form[
  #set form.textbox(max-length: 5, width: 100pt)
  // Error: 4-33 textbox's value is longer than `max-length`
  // Hint: 4-33 the value has `6` characters, but the maximum is `5` characters
  #form.textbox(value: "abcdef")
  // Error: 4-35 textbox's value is longer than `max-length`
  // Hint: 4-35 the value has `8` characters, but the maximum is `5` characters
  #form.textbox(value: "分别设置分别设置")
  // Error: 4-33 textbox's value is longer than `max-length`
  // Hint: 4-33 the value has `6` characters, but the maximum is `5` characters
  #form.textbox(value: "שנקבעו")
]

--- form-textbox-placeholder-paged paged ---
#form[
  // Warning: 4-53 `placeholder` property was ignored during paged export
  // Hint: 4-53 the property is not supported in this export format
  #form.textbox(placeholder: "foobar", width: 100pt)
]

--- form-textbox-placeholder-html html ---
#form[
  #form.textbox(placeholder: "foobar")
]

--- form-place-circle-on-textbox paged ---
// TODO: needs fixing, probably png and svg export should match pdf
#form[
  #form.textbox(width: 100pt, value: "foobar foobar")
  #place(dy: -20pt, circle(fill: red))
]

--- form-textbox-chinese-value paged ---
#form[
  #form.textbox(value: "分别设置", width: 100pt)
]

--- form-textbox-hebrew-value paged ---
#form[
  #form.textbox(value: "שנקבעו", width: 100pt)
]

--- form-choice-non-latin-options paged ---
#form[
  #let options = (
    "foo": "bar",
    "分别": "设置",
    "שנק": "בעו",
  )
  #form.choice(options: options, value: "分别")
  #form.choice(multiple: true, options: options, value: ("foo", "שנק"))
]

--- form-radio-group-non-latin-values paged ---
#form[
  #form.radio-group(selected: <opt-2>)[
    #form.radio(value: "foo")
    #form.radio(value: "分别") <opt-2>
    #form.radio(value: "שנק")
  ]
]

--- form-textbox-auto-default-font paged ---
// Check that the most similar standard font is chosen based on the current font.
// NOTE: The resulting PDF needs to be manually checked.

#form[
  #set form.textbox(width: 100pt, value: "Lorem Ipsum")

  #set text(font: "Roboto Flex")
  #set text(weight: "regular", style: "normal")
  #form.textbox() // Helvetica
  #set text(weight: "bold", style: "normal")
  #form.textbox() // Helvetica Bold
  #set text(weight: "regular", style: "italic")
  #form.textbox() // Helvetica Italic
  #set text(weight: "bold", style: "italic")
  #form.textbox() // Helvetica BoldItalic

  #set text(font: "Source Serif 4")
  #set text(weight: "regular", style: "normal")
  #form.textbox() // Times Roman
  #set text(weight: "bold", style: "normal")
  #form.textbox() // Times Bold
  #set text(weight: "regular", style: "italic")
  #form.textbox() // Times Italic
  #set text(weight: "bold", style: "italic")
  #form.textbox() // Times BoldItalic

  #set text(font: "DejaVu Sans Mono")
  #set text(weight: "regular", style: "normal")
  #form.textbox() // Courier
  #set text(weight: "bold", style: "normal")
  #form.textbox() // Courier Bold
  #set text(weight: "regular", style: "italic")
  #form.textbox() // Courier Italic
  #set text(weight: "bold", style: "italic")
  #form.textbox() // Courier BoldItalic
]

--- form-widget-show-rule paged ---
#form[
  #show form.button: it => {
    // Replace the default rect with something else
    // Set different appearances depending on interactive state
    show form.widget.where(state: "normal"): ellipse(fill: red, it.body)
    show form.widget.where(state: "hovered"): ellipse(fill: purple, it.body)
    show form.widget.where(state: "down"): ellipse(fill: yellow, it.body)

    it
  }

  #form.button[Reset form]

  #show form.checkbox: it => {
    // Set different appearances depending on interactive state and whether it is checked
    show form.widget.where(checked: false, state: "normal"): set rect(fill: red)
    show form.widget.where(checked: true, state: "normal"): set rect(fill: green)
    show form.widget.where(checked: false, state: "hovered"): set rect(fill: purple)
    show form.widget.where(checked: true, state: "hovered"): set rect(fill: teal)
    show form.widget.where(state: "down"): set rect(fill: yellow)

    box(it)
  }

  #form.checkbox() <checkbox-1>
  #form.label(<checkbox-1>)[Foo]

  #form.checkbox(checked: true) <checkbox-2>
  #form.label(<checkbox-2>)[Bar]

  #show form.radio: it => {
    // Replace widget with a rect instead of circle
    set rect(width: 100%, height: 100%, inset: 0.2em)
    show form.widget.where(checked: false): rect[]
    show form.widget.where(checked: true): rect(rect(fill: black))

    box(it)
  }

  #form.radio-group(selected: <radio-2>)[
    #form.radio() <radio-1>
    #form.label(<radio-1>)[Foo]

    #form.radio() <radio-2>
    #form.label(<radio-2>)[Bar]
  ]

  #show form.textbox: it => {
    // Change text color (readers should then use it when editing the field too)
    set text(fill: blue)
    set rect(stroke: 1pt + gradient.linear(red, green), outset: -1pt)
    set align(right)
    show form.widget: it => {
      // It is possible to change the text here in a way that does not match the value.
      // Readers might then wrongly assume this is the actual value, however.
      show "foobar": [barfoo #emoji.face]
      it
    }
    it
  }

  #form.textbox(width: 100pt, value: "foobar")

  #let options = ("foo", "bar", "baz")

  #show form.choice: it => {
    // Replace the arrow on the right side with an emoji
    show sym.triangle.filled.small.b: emoji.arrow.b.filled

    set rect(stroke: 0.5pt)
    it
  }

  #form.choice(options: options, value: "foo")
]

--- form-widget-emit-down-when-hovered-is-different paged ---
// Pdfium only recognizes hover/down appearances if all three of normal/hover/down are set.
// If only two of the three are emitted, Pdfium will only use the normal appearance.
// NOTE: needs manual inspection of the PDF and/or testing in Pdfium.

#form[
  #show form.button: it => {
    // Normal and down are the same
    // NOTE: hover/down in Pdfium does not work on buttons anyway
    show form.widget: ellipse(fill: red, it.body)
    show form.widget.where(state: "hovered"): ellipse(fill: purple, it.body)

    it
  }

  #form.button[Button]

  #show form.checkbox: it => {
    // Normal and down are the same when unchecked
    show form.widget.where(checked: false): set rect(fill: red)
    show form.widget.where(checked: false, state: "hovered"): set rect(fill: purple)
    // All 3 states are the same when checked
    show form.widget.where(checked: true): set rect(fill: green)
    it
  }

  #form.checkbox()

  #show form.radio: it => {
    // All 3 states are the same when unchecked
    show form.widget.where(checked: false): set circle(fill: red)
    show form.widget.where(checked: true): set circle(fill: green)
    // Normal and hover are the same when checked
    show form.widget.where(checked: true, state: "down"): set circle(fill: yellow)
    it
  }

  #form.radio-group[
    #form.radio()
    #form.radio()
  ]
]

--- form-transformations paged ---
#form[
  #set scale(reflow: true)
  #set rotate(reflow: true)
  #set skew(reflow: true)
  #set form.checkbox(checked: true)

  #form.button(lorem(2))
  #rotate(30deg, form.button(lorem(2)))

  #form.checkbox()
  #scale(-150%, form.checkbox())

  #form.radio-group(selected: <radio>)[
    #form.radio()
    #skew(ax: 30deg)[#form.radio() <radio>]
  ]
]

--- form-transformations-variable-text-rotate pdf ---
#form[
  #form.textbox()
  // Error: 18-32 textboxes and choice fields cannot be scaled, skewed, or rotated when exporting to PDF
  // Hint: 18-32 remove any surrounding `scale`, `skew`, or `rotate` elements
  #rotate(30deg, form.textbox())
]

--- form-transformations-variable-text-scale pdf ---
#form[
  #form.textbox()
  // Error: 17-31 textboxes and choice fields cannot be scaled, skewed, or rotated when exporting to PDF
  // Hint: 17-31 remove any surrounding `scale`, `skew`, or `rotate` elements
  #scale(-150%, form.textbox())
]

--- form-transformations-variable-text-skew pdf ---
#form[
  #form.choice()
  // Error: 20-33 textboxes and choice fields cannot be scaled, skewed, or rotated when exporting to PDF
  // Hint: 20-33 remove any surrounding `scale`, `skew`, or `rotate` elements
  #skew(ax: 30deg, form.choice())
]

--- form-nested-fields paged html ---
#form[
  // Hint: 1:4-8:4 the outer form field is here
  // Hint: 1:4-8:4 the outer form field is here
  // Hint: 1:4-8:4 the outer form field is here
  // Hint: 1:4-8:4 the outer form field is here
  #form.button[
    // Error: 6-34 checkbox element cannot occur inside of another form field
    #form.checkbox(checked: true)
    #form.radio-group[
      // Error: 8-20 radio element cannot occur inside of another form field
      #form.radio()
    ]
    // Error: 6-20 textbox element cannot occur inside of another form field
    #form.textbox()
    // Error: 6-19 choice element cannot occur inside of another form field
    #form.choice()
  ]

  #show form.checkbox: [
    // Error: 6-19 button element cannot occur inside of another form field
    #form.button[]
  ]
  // Hint: 4-19 the outer form field is here
  #form.checkbox()
]

--- form-nested-forms paged html ---
// Hint: 1:2-3:2 the outer form is here
#form[
  // Error: 4-10 this form cannot be inside another form
  #form[]
]

--- form-multiple paged html ---
#form(target: "https://example.com")[
  // Use `.` so that partial field name != fully qualified field name
  #form.checkbox(name: "checkbox.1")
  #form.textbox(name: "textbox-1", width: 100pt)

  // NOTE: needs checking manually on PDF that
  // the buttons only target the fields above
  #form.button(action: "reset")[Reset]
  #form.button(action: "submit")[Submit]
]

#form(target: "https://example.net")[
  #form.checkbox(name: "checkbox.2")
  #form.textbox(name: "textbox-2", width: 100pt)

  // NOTE: needs checking manually on PDF that
  // the buttons only target the fields above
  #form.button(action: "reset")[Reset]
  #form.button(action: "submit")[Submit]
]

--- form-partial-field-names pdf ---
// On PDF, the dot (`.`) is a special character that creates hierarchy in the field tree.
// Test whether that hierarchy is correctly created.
// NOTE: needs manual checking

#form[
  #form.checkbox(name: "foo.bar.baz.qux.quux")
  #form.checkbox(name: "foo.bar.baz.foo")
  #form.checkbox(name: "foo.bar.baz.bar")
  #form.checkbox(name: "bar.bar")
  #form.checkbox(name: "qux")
]

--- form-button paged html ---
#form[
  #form.button[No action]
  #form.button(name: "custom-name")[Custom name]
  #form.button(action: "reset")[Reset action]
  #form.button(disabled: true)[Disabled]
  #form.button(width: 100pt, height: 40pt)[Custom size]
]

--- form-checkbox paged html ---
#form[
  #form.label(<cb>)[Unchecked:]
  #form.checkbox() <cb>
  #form.label(<cb-checked>)[Checked:]
  #form.checkbox(checked: true) <cb-checked>

  #form.label(<cb-disabled>)[Unchecked disabled:]
  #form.checkbox(disabled: true) <cb-disabled>
  #form.label(<cb-checked-disabled>)[Checked disabled:]
  #form.checkbox(checked: true, disabled: true) <cb-checked-disabled>

  #form.label(<cb-required>)[Unchecked required:]
  #form.checkbox(required: true) <cb-required>
  #form.label(<cb-checked-required>)[Checked required:]
  #form.checkbox(checked: true, required: true) <cb-checked-required>

  #form.label(<cb-named>)[Named:]
  #form.checkbox(name: "checkbox-1") <cb-named>

  #form.label(<cb-size>)[Custom size:]
  #form.checkbox(width: 15pt, height: 20pt, checked: true) <cb-size>
]

--- form-radio paged html ---
#form[
  Without title:
  #form.radio-group(selected: <radio-1>)[
    #form.radio(value: "foo") <radio-1>
    #form.label(<radio-1>)[Option 1]

    #form.radio() <radio-2>
    #form.label(<radio-2>)[Option 2]
  ]

  With title and required:
  #form.radio-group(required: true, title: [This is a title])[
    #form.radio() <radio-3>
    #form.label(<radio-3>)[Option 1]

    #form.radio(width: 15pt, height: 20pt) <radio-4>
    #form.label(<radio-4>)[Option 2]
  ]

  Disabled:
  #form.radio-group(disabled: true)[
    #form.radio() <radio-5>
    #form.label(<radio-5>)[Option 1]

    #form.radio() <radio-6>
    #form.label(<radio-6>)[Option 2]
  ]
]

--- form-textbox paged html ---
#set form.textbox(width: 100pt)

#form[
  #form.label(<tb>)[Default:]
  #form.textbox() <tb>

  #form.label(<tb-value>)[With value:]
  #form.textbox(value: "lorem ipsum", spellcheck: false) <tb-value>

  #form.label(<tb-required>)[Required:]
  #form.textbox(required: true, spellcheck: true) <tb-required>

  #form.label(<tb-disabled>)[Disabled:]
  #form.textbox(disabled: true, value: "lorem ipsum") <tb-disabled>

  #form.label(<ta>)[Multiline:]
  #form.textbox(kind: "multi-line", height: 2em) <ta>

  #form.label(<ta-value>)[Multiline With value:]
  #form.textbox(kind: "multi-line", value: "lorem\nipsum", spellcheck: false) <ta-value>

  #form.label(<ta-required>)[Multiline required:]
  #form.textbox(kind: "multi-line", required: true, spellcheck: true) <ta-required>

  #form.label(<ta-disabled>)[Disabled:]
  #form.textbox(kind: "multi-line", disabled: true, value: "lorem\nipsum") <ta-disabled>
]

--- form-choice paged html ---
#form[
  #let options = ("opt1": "Option 1", "opt2": "Option 2", "opt3": "Option 3")
  #set form.choice(options: options)

  #form.label(<dd>)[Dropdown:]
  #form.choice() <dd>

  #form.label(<dd-value>)[Dropdown with value:]
  #form.choice(value: "opt2") <dd-value>

  #form.label(<dd-required>)[Dropdown with required:]
  #form.choice(required: true) <dd-required>

  #form.label(<dd-custom-size>)[Dropdown with custom size:]
  #form.choice(width: 60pt, height: 30pt) <dd-custom-size>

  #form.label(<dd-disabled>)[Dropdown with disabled:]
  #form.choice(value: "opt1", disabled: true) <dd-disabled>

  #form.label(<lb>)[Listbox:]
  #form.choice(multiple: true) <lb>

  #form.label(<lb-value>)[Listbox with value:]
  #form.choice(multiple: true, value: "opt2") <lb-value>

  #form.label(<lb-values>)[Listbox with multiple values:]
  #form.choice(multiple: true, value: ("opt1", "opt3")) <lb-values>

  #form.label(<lb-required>)[Listbox with required:]
  #form.choice(multiple: true, required: true) <lb-required>

  #form.label(<lb-custom-size>)[Listbox with custom size:]
  #form.choice(multiple: true, width: 60pt, height: 30pt) <lb-custom-size>

  #form.label(<lb-disabled>)[Listbox with disabled:]
  #form.choice(multiple: true, value: "opt1", disabled: true) <lb-disabled>
]

--- form-bundle bundle ---
#document("index.html")[
  #form[
    #form.label(<cb-1>)[Checkbox:]
    #form.checkbox() <cb-1>
  ]
]

#document("index.pdf")[
  #form[
    #form.label(<cb-2>)[Checkbox:]
    #form.checkbox() <cb-2>
  ]
]

--- form-counter paged ---
// Use PDF A/4 to ensure hover/down widgets get filtered out correctly.
#set pdf(standard: "a-4")
#set document(date: datetime(year: 1970, month: 1, day: 1))

#form[
  #let cnt = counter("counter-1")

  #show form.checkbox: it => {
    show form.widget.where(checked: false): it => {
      cnt.step()
      rect[
        unchecked
        #context cnt.display()
      ]
    }
    show form.widget.where(checked: true): it => {
      ellipse(align(center + horizon)[
        checked
        #cnt.step()#context cnt.display()
      ])
    }
    it
  }
  #set form.checkbox(width: 10em)

  #context cnt.display() // 0

  #form.checkbox() // unchecked 1; checked 0 (FIXME?)

  #context cnt.display() // 1

  #cnt.step()
  #context cnt.display() // 2

  #form.checkbox(checked: true) // unchecked 3; checked 2 (FIXME?)

  #context cnt.display() // 3

  #cnt.step()
  #context cnt.display() // 4
]

--- form-counter-same-widget paged ---
// Use PDF A/4 to ensure hover/down widgets get filtered out correctly.
#set pdf(standard: "a-4")
#set document(date: datetime(year: 1970, month: 1, day: 1))

#form[
  #let cnt = counter("counter-1")

  #show form.checkbox: it => {
    show form.widget: it => {
      cnt.step()
      rect[
        #if not it.checked [un];checked
        #context cnt.display()
      ]
    }
    it
  }
  #set form.checkbox(width: 10em)

  #form.checkbox() // unchecked 1; checked 1
  #form.checkbox() // unchecked 2; checked 2

  #context cnt.display() // 2
]

--- form-counter-html-frame html ---
#form[
  #let cnt = counter("counter-1")

  #show form.checkbox: it => {
    show form.widget.where(checked: false): it => {
      cnt.step()
      it
    }
    it
  }
  #context cnt.display() // 0
  #form.checkbox()
  #context cnt.display() // 0
  #html.frame(form.checkbox())
  #context cnt.display() // 1
]

--- form-hide paged ---
#form[
  #hide[
    #form.textbox() <textbox>
  ]
  #form.label(<textbox>)[Hidden Textbox]
]

--- form-gradient-fill paged ---
// Test shows a checkbox inside a block filled with a gradient.
// The fill of the checkbox should match the block behind it, as if it were transparent.
// Then, there are a checkbox and textbox on the page placed against a white background,
// and they should filled with the gradient as well, but relative to the page itself.
#let grad = gradient.linear(red, blue, green, purple, relative: "parent")
#show form.checkbox: set rect(fill: grad)
#set form.checkbox(checked: true)
#show form.textbox: set rect(fill: grad)
#form[
  #rotate(reflow: true, 10deg)[
    #block(width: 70pt, height: 60pt, fill: grad)[
      #place(dy: 20pt, dx: 30pt, rotate(30deg, reflow: true, form.checkbox()))
    ]
  ]
  #form.checkbox()
  #form.textbox(width: 100pt)
]

--- form-rect-in-stylechain paged ---
// Textboxes and dropdowns partially set the rect's stroke.
// Ensure that the sides with no stroke actually remove it,
// even if there is a set rule for rect the form field.
#form[
  #set form.textbox(width: 100pt)
  #set rect(stroke: 2pt + red)

  // bottom stroke should be red, but still 0.5pt
  // other sides should have no stroke
  #form.textbox()
  #form.choice()

  #show form.textbox: set rect(stroke: 5pt)
  #show form.choice: set rect(stroke: 5pt)
  // all sides should have 5pt + red stroke
  #form.textbox()
  #form.choice()
]

--- form-textbox-overflow paged ---
#form[
  #form.textbox(width: 15pt, value: "aaaaaaaaaaaaaa")
]
