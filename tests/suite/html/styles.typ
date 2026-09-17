--- html-styles-options-precedence bundle ---
#let content-that-has-styles = [
  #underline[abc]
  #smallcaps[def]
]

// Tests the precedence of different ways to specify document format options.
#set html(styles: (path: "styles.css"))

// Should contain an embedded stylesheet.
#document("a.html", {
  set html(styles: "embedded")
  content-that-has-styles
})

// Should contain a link to the `styles.css` stylesheet
#document("b.html", {
  content-that-has-styles
})

// Should contain a link to the `other.css` stylesheet
#document("c.html", {
  set html(styles: (path: "other.css"))
  content-that-has-styles
})

// Should contain inline stylesheet.
#set html(styles: "inline")
#document("d.html", {
  content-that-has-styles
})

--- html-styles-none html ---
#set html(styles: none)

// Discard all styles, also semantic ones.
#underline[abc]
#smallcaps[def]
