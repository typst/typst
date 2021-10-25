// Link without body.
#link("https://example.com/")

// Link with body.
#link("https://typst.app/")[Some text text text]

// With line break.
This link appears #link("https://google.com/")[in the middle of] a paragraph.

// Prefix is trimmed.
Contact #link("mailto:hi@typst.app") or call #link("tel:123") for more information.

---
// Styled with underline and color.
#let link(url, body) = link(url, font(fill: rgb("283663"), underline(body)))
You could also make the #link("https://html5zombo.com/")[link look way more typical.]
