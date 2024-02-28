// Tests that when a citation footnote is pushed to next page, things still
// work as expected.
//
// Issue: https://github.com/typst/typst/issues/1597

---
#set page(height: 60pt)
#lorem(4)

#footnote[@netwok]
#show bibliography: none
#bibliography("/assets/bib/works.bib")
