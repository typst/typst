/// Issue #2821: Setting a figure's supplement to none removes the field
// Ref: false

---
#show figure.caption: it => {
  assert(it.has("supplement"))
  assert(it.supplement == none)
}
#figure([], caption: [], supplement: none)
