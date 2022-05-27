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
      if item.value in seen { continue }
      (num(all, item.value), item.value)
      seen.push(item.value)
    }
  }
))}

As shown in #cite("abc") and #cite("def") and #cite("abc") ...

---
// Test lovely sidebar.
#let lovely = group("lovely")
#let words = ("Juliet", "soft", "fair", "maid")
#let regex = regex(words.map(p => "(" + p + ")").join("|"))
#show word: regex as underline(word) + lovely.entry(_ => {})
#set page(
  paper: "a8",
  margins: (left: 25pt, rest: 15pt),
  foreground: lovely.all(entries => {
    let seen = ()
    for y in entries.map(it => it.y) {
      if y in seen { continue }
      let line = entries.filter(it => it.y == y)
      for i, it in line {
        let x = 10pt - 4pt * (line.len() - i - 1)
        place(dx: x, dy: it.y - 8pt, [💗])
      }
      seen.push(y)
    }
  }),
)

But, soft! what light through yonder window breaks? It is the east, and Juliet
is the sun. Arise, fair sun, and kill the envious moon, Who is already sick and
pale with grief, That thou her maid art far more fair than she: Be not her maid,
since she is envious.

---
// Test that `all` contains `me`.
// Ref: false
#show it: heading as group("headings").entry(
  (me, all) => {
    let last
    for prev in all {
      last = prev
      if prev.index == me.index {
        break
      }
    }
    assert(last == me)
  }
)

= A
== B
