#let conf(title: "", style: "", content) = {
  html.elem("html", attrs: ("lang": "en"))[
    #html.head[
      #html.meta(charset: "utf-8")
      #html.title[#title]
      #html.style(style)
    ]
    #html.body(content)
  ]
}

#let diff-line(kind, line-nr, spans) = {
  html.td(class: "line-gutter diff-" + kind, if line-nr != 0 [ #line-nr ])
  html.td(class: "line-body diff-" + kind)[
    #html.pre(class: "line-text")[
      #for span in spans {
        if span.emph and kind in ("add", "del"){
          html.span(class: "span-" + kind, [#span.text])
        } else {
          [#span.text]
        }
      }
    ]
  ]
}

#let diff-cells(line) = {
  if line.kind in ("empty","del", "add", "unchanged", "end") {
    diff-line(line.kind, line.nr, line.spans)
  } else if line.kind == "gap" {
    html.td(colspan: 2, class: "diff-gap")[#sym.dots.h.c]
  } else {
    panic("unhandled kind: " + line.kind)
  }
}

#show: conf.with(
  title: "Typst test report",
  style: sys.inputs.style
)

#html.div(class: "container")[
  #html.div(class: "sidebar-container")[
    #html.div(class: "sidebar")[
      #for test in sys.inputs.diffs {
        link("#" + test.name)[#test.name]
      }
    ]
  ]
  #html.div(class: "diff-container")[
    #for diff in sys.inputs.diffs {
      html.div(class: "file-diff", id: diff.name)[
        #html.input(type: "checkbox", class: "collapse-diff")
        #html.table(class: "diff-area")[
          #html.colgroup[
            #html.col(span: 1, class: "col-line-gutter")
            #html.col(span: 1, class: "col-line-body")
            #html.col(span: 1, class: "col-line-gutter")
            #html.col(span: 1, class: "col-line-body")
          ]

          #html.thead(class: "diff-header")[
            #html.tr[
              #html.th(colspan: 2)[
                #link("../../" + diff.left.path)[#diff.left.path]
              ]
              #html.th(colspan: 2)[
                #link("../../" + diff.right.path)[#diff.right.path]
              ]
            ]
          ]

          #html.tbody(class: "diff-body")[
            #for (l, r) in diff.left.lines.zip(diff.right.lines){
              html.tr(class: "diff-line")[
                #diff-cells(l)
                #diff-cells(r)
              ]
            }
            #html.tr(class: "diff-line")[
              #html.td(colspan: 4, class: "diff-end")
            ]
          ]
        ]
      ]
    }
  ]
]
