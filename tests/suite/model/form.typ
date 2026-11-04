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
  #form.textbox(multiline: true) <textarea>

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
  #form.textbox(multiline: true) <textarea>

  #form.label(<dropdown>)[Dropdown]
  #form.choice(options: ("foo", "bar")) <dropdown>

  #form.label(<listbox>)[Listbox]
  #form.choice(options: ("foo", "bar"), multiple: true) <listbox>
]

--- form-target-invalid-content-type-html html ---
// HTML cannot export to FDF, XFDF, nor PDF.
// Error: 2-67 content-type `"fdf"` is not supported in HTML export
#form(target: (url: "https://example.com", content-type: "fdf"))[]
// Error: 2-68 content-type `"xfdf"` is not supported in HTML export
#form(target: (url: "https://example.com", content-type: "xfdf"))[]
// Error: 2-67 content-type `"pdf"` is not supported in HTML export
#form(target: (url: "https://example.com", content-type: "pdf"))[]

--- form-target-invalid-content-type-pdf pdf ---
// PDF cannot export to multipart nor plain text.
// Error is only triggered when there is a submit button.

#form(target: (url: "https://example.com", content-type: "multipart"))[
  // Error: 4-35 content-type `"multipart"` is not supported in PDF export
  #form.button(action: "submit")[]
]

#form(target: (url: "https://example.com", content-type: "plain"))[
  // Error: 4-35 content-type `"plain"` is not supported in PDF export
  #form.button(action: "submit")[]
]

--- form-target-warning-content-type-pdf pdf ---
// PDF readers have poor support for all formats that are not FDF.
// Warning is only triggered when there is a submit button.

#form(target: (url: "https://example.com", content-type: "url-encoded"))[
  // Warning: 4-41 content-type `"url-encoded"` has poor support among PDF readers
  // Hint: 4-41 prefer the `"fdf"` content-type whenever possible
  #form.button(action: "submit")[Submit]
]

#form(target: (url: "https://example.com", content-type: "xfdf"))[
  // Warning: 4-41 content-type `"xfdf"` has poor support among PDF readers
  // Hint: 4-41 prefer the `"fdf"` content-type whenever possible
  #form.button(action: "submit")[Submit]
]

#form(target: (url: "https://example.com", content-type: "pdf"))[
  // Warning: 4-41 content-type `"pdf"` has poor support among PDF readers
  // Hint: 4-41 prefer the `"fdf"` content-type whenever possible
  #form.button(action: "submit")[Submit]
]

--- form-target-method-get-pdf pdf ---
// PDF cannot use GET method.
// Error is only triggered when there is a submit button.
#form(target: (url: "https://example.com", method: "get"))[
  // Error: 4-35 method `"GET"` is not supported in PDF export
  #form.button(action: "submit")[]
]

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

--- form-cursor-outside-textbox paged ---
// Error: 2-15 this cursor element must be inside a textbox element
// Hint: 2-15 you can use it in a show rule such as ```\n#show form.textbox: {\n   show form.widget: form.cursor()\n   it\n}\n```
#form.cursor()

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
  // Error: 1:4-3:4 duplicate option
  // Hint: 1:4-3:4 occurred in option at index 2 (`"foo"`)
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
  #show form.textbox: it => {
    show form.cursor: set text(fill: color)
    it
  }
  // Error: 4-18 text of a form field can only be filled with a solid color
  // Hint: 4-18 remove the gradient or pattern
  #form.textbox()

  #show form.choice: it => {
    set text(fill: color)
    it
  }

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
  // Error: 4-30 two fields share the same name ("foo")
  // Hint: 4-30 change the name of one of the fields
  #form.checkbox(name: "foo")
]

--- form-duplicate-fully-qualified-field-names pdf ---
#form[
  // Error: 4-34 there are two distinct form fields named `foo.bar`
  // Hint: 4-34 this can happen if you have a field named `foo.bar` and another named `foo.bar.something-else`
  #form.checkbox(name: "foo.bar")
  #form.checkbox(name: "foo.bar.baz")
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
  // Hint: 4-18 make sure your form fields have a label
  // Hint: 4-18 add a `#form.label` that points to this field
  // Hint: 4-18 for `#form.radio-group`, you can set the `title` property instead
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
  // Hint: 4-19 remove `show form.widget.where(state: ...)` rules
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
  // Hint: 4-19 remove `show form.widget.where(state: ...)` rules
  #form.checkbox() <checkbox-2>
]

--- form-textbox-fixed-width-errors paged ---
#form[
  // Error: 4-52 `fixed-width` cannot be `true` for multiline textboxes
  #form.textbox(multiline: true, fixed-width: true)
  // Error: 4-35 `max-length` must be set when `fixed-width` is `true`
  #form.textbox(fixed-width: true)
]

--- form-textbox-fixed-width-paged paged ---
#form[
  #form.textbox(fixed-width: true, max-length: 9, width: 100pt)
]

--- form-textbox-fixed-width-html html ---
#form[
  // Warning: 4-35 `fixed-width` property was ignored during HTML export
  // Hint: 4-35 the property is not supported in this export format
  #form.textbox(fixed-width: true)
]

--- form-textbox-placeholder-paged paged ---
#form[
  // Warning: 4-53 ignored `placeholder` property when exporting
  // Hint: 4-53 property not supported in this export format
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
    show form.widget: it => {
      set rect(stroke: 1pt + gradient.linear(red, green), outset: -1pt)
      // TODO: align doesn't work
      align(right, it)
    }
    show form.cursor: it => {
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

    rect(stroke: (bottom: 0.5pt, rest: none), inset: 0pt, it)
  }

  #form.choice(options: options, value: "foo")
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
    // Error: 6-34 this checkbox element cannot be inside a form field
    #form.checkbox(checked: true)
    #form.radio-group[
      // Error: 8-20 this radio element cannot be inside a form field
      #form.radio()
    ]
    // Error: 6-20 this textbox element cannot be inside a form field
    #form.textbox()
    // Error: 6-19 this choice element cannot be inside a form field
    #form.choice()
  ]

  #show form.checkbox: [
    // Error: 6-19 this button element cannot be inside a form field
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
  #form.label(<rg>)[Without title:]
  #form.radio-group(selected: <radio-1>)[
    #form.radio(value: "foo") <radio-1>
    #form.label(<radio-1>)[Option 1]

    #form.radio() <radio-2>
    #form.label(<radio-2>)[Option 2]
  ] <rg>

  #form.label(<rg-title>)[With title and required:]
  #form.radio-group(required: true, title: [This is a title])[
    #form.radio() <radio-3>
    #form.label(<radio-3>)[Option 1]

    #form.radio(width: 15pt, height: 20pt) <radio-4>
    #form.label(<radio-4>)[Option 2]
  ] <rg-title>

  #form.label(<rg-disabled>)[Disabled:]
  #form.radio-group(disabled: true)[
    #form.radio() <radio-5>
    #form.label(<radio-5>)[Option 1]

    #form.radio() <radio-6>
    #form.label(<radio-6>)[Option 2]
  ] <rg-disabled>
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
  #form.textbox(multiline: true, height: 2em) <ta>

  #form.label(<ta-value>)[Multiline With value:]
  #form.textbox(multiline: true, value: "lorem\nipsum", spellcheck: false) <ta-value>

  #form.label(<ta-required>)[Multiline required:]
  #form.textbox(multiline: true, required: true, spellcheck: true) <ta-required>

  #form.label(<ta-disabled>)[Disabled:]
  #form.textbox(multiline: true, disabled: true, value: "lorem\nipsum") <ta-disabled>
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
  #form.choice(required: true, width: 60pt, height: 30pt) <dd-required>

  #form.label(<dd-disabled>)[Dropdown with disabled:]
  #form.choice(value: "opt1", disabled: true) <dd-disabled>

  #form.label(<lb>)[Listbox:]
  #form.choice(multiple: true) <lb>

  #form.label(<lb-value>)[Listbox with value:]
  #form.choice(multiple: true, value: "opt2") <lb-value>

  #form.label(<lb-values>)[Listbox with multiple values:]
  #form.choice(multiple: true, value: ("opt1", "opt3")) <lb-values>

  #form.label(<lb-required>)[Listbox with required:]
  #form.choice(multiple: true, required: true, width: 60pt, height: 30pt) <lb-required>

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
// TODO: we need to revisit how counters should work in the widgets

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
