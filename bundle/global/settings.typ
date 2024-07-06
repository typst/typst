#let get-number() = {
  let file = "/" + entrypoint().split("/").slice(0, -1).join("/") + "/number"
  read(file).find(regex("\d+"))
}

#let set-document(doc, number: auto) = {
  set document(title: [Document #if number == auto {
      "№ " + get-number()
    } else {
      if type(number) != int { "№" } + "№ " + number
    }])
  doc
}
#let set-page(doc) = {
  set page(paper: "a5")
  doc
}
// #let set-bibliography = bibliography("/global/bibliography/main.yaml")
#let set-bibliography
#let set-rules(doc) = {
  // ...
  doc
}
#let show-rules(doc) = {
  // ...
  doc
}
#let settings(doc) = {
  show: set-rules
  show: show-rules

  show: it => {
    let is-bundle-document = entrypoint() == "/src/main.typ"
    if is-bundle-document { return it }
    show: set-document
    show: set-page
    set-bibliography
    it
  }

  // show: turn-on-first-line-indentation
  doc
}
