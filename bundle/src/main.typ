#(import "/global/settings.typ":
set-document,
set-page,
show-rules,
set-rules,
set-bibliography)

#let total = 2
#show: set-rules
#show: show-rules
#show: set-document.with(number: "1" + if total > 1 [--#total])
#show: set-page
#set-bibliography

Bundle document

#{
  let path = raw(entrypoint())
  `(entrypoint() == "` + path + `")`
}

#for n in range(1, total + 1) {
  include "sub" + str(n) + "/src/main.typ"
}
