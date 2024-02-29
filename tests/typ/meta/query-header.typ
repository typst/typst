// Test creating a header with the query function.

---
#set page(
  paper: "a7",
  margin: (y: 1cm, x: 0.5cm),
  header: context {
    smallcaps[Typst Academy]
    h(1fr)
    let after = query(selector(heading).after(here()))
    let before = query(selector(heading).before(here()))
    let elem = if before.len() != 0 {
      before.last()
    } else if after.len() != 0 {
      after.first()
    }
    emph(elem.body)
  }
)

#outline()

= Introduction
#lorem(35)

= Background
#lorem(35)

= Approach
#lorem(60)
