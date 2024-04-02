// Smart quotes were not appearing in the PDF outline, because they didn't
// implement `PlainText`
// https://github.com/typst/typst/issues/3662

---
= It's "Unnormal Heading"
= It’s “Normal Heading”

#set smartquote(enabled: false)
= It's "Unnormal Heading"
= It's 'single quotes'
= It’s “Normal Heading”