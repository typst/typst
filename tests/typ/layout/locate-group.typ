// Test locatable groups.

---
// Test counting.
#let letters = group("\u{1F494}")
#let counter = letters.entry(
  (me, all) => [{1 + me.index} / {all.len()}]
)

#counter \
#box(counter) \
#counter \

---
// Test minimal citation engine with references before the document.
#let cited = group("citations")
#let num(cited, key) = {
  let index = 0
  for item in cited {
    if item.value == key {
      index = item.index
      break
    }
  }
  [\[{index + 1}\]]
}

#let cite(key) = cited.entry(value: key, (_, all) => num(all, key))
{cited.all(all => grid(
  columns: (auto, 1fr),
  gutter: 5pt,
  ..{
    let seen = ()
    for item in all {
      if item.value not in seen {
        seen.push(item.value)
        (num(all, item.value), item.value)
      }
    }
  }
))}

As shown in #cite("abc") and #cite("def") and #cite("abc") ...
