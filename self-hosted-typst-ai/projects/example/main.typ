// Example Typst Document
// This demonstrates common Typst patterns the AI can generate

#set document(
  title: "My Document",
  author: "Your Name",
  date: datetime.today(),
)

#set page(
  paper: "a4",
  margin: (x: 2.5cm, y: 2.5cm),
  header: [
    #set text(9pt)
    _My Document_ #h(1fr) #counter(page).display()
  ],
)

#set text(
  font: "Linux Libertine",
  size: 11pt,
)

#set heading(numbering: "1.1")

// Title
#align(center)[
  #text(size: 24pt, weight: "bold")[My Document Title]
  #v(0.5em)
  #text(size: 12pt)[Your Name]
  #v(0.5em)
  #text(size: 10pt, fill: gray)[#datetime.today().display()]
]

#v(2em)

= Introduction

This is a sample document created with Typst. You can ask the AI to generate
content like this by describing what you want in natural language.

== What You Can Ask For

Try prompts like:
- "Add a table with columns for name, age, and city with sample data"
- "Create a bullet list of project goals"
- "Add a math equation for the quadratic formula"
- "Insert a code block showing a Python function"

= Example Table

#table(
  columns: (1fr, auto, 1fr),
  inset: 10pt,
  align: horizon,
  table.header(
    [*Name*], [*Age*], [*City*],
  ),
  [Alice], [28], [New York],
  [Bob], [34], [San Francisco],
  [Carol], [45], [Chicago],
)

= Example Math

The quadratic formula:

$ x = (-b plus.minus sqrt(b^2 - 4 a c)) / (2 a) $

= Example Code

```python
def greet(name: str) -> str:
    """Return a greeting message."""
    return f"Hello, {name}!"
```

= Conclusion

This document demonstrates basic Typst features. Use the AI assistant to
add more content, modify styling, or create entirely new documents.
