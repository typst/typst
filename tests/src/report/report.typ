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
        if span.emph and kind == "del" {
          html.del([#span.text])
        } else if span.emph and kind == "add" {
          html.ins([#span.text])
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
      #html.ul(class: "sidebar-list")[
        #for test in sys.inputs.diffs {
          html.li[
            #link("#" + test.name)[#test.name]
          ]
        }
      ]
    ]
  ]
  #html.div(class: "diff-container")[
    #for diff in sys.inputs.diffs {
      html.div(class: "file-diff", id: diff.name)[
        #html.details(open: true)[
          #html.summary(class: "diff-summary")[
            #html.div(class: "diff-spacer")
            #html.h1(class: "diff-header")[
              #html.div(class: "diff-header-split")[
                *#link("../../" + diff.left.path)[#diff.left.path]*
              ]
              #html.div(class: "diff-header-split")[
                *#link("../../" + diff.right.path)[#diff.right.path]*
              ]
            ]
          ]

          #html.table(class: "diff-area")[
            #html.colgroup[
              #html.col(span: 1, class: "col-line-gutter")
              #html.col(span: 1, class: "col-line-body")
              #html.col(span: 1, class: "col-line-gutter")
              #html.col(span: 1, class: "col-line-body")
            ]

            #for (l, r) in diff.left.lines.zip(diff.right.lines){
              html.tr(class: "diff-line")[
                #diff-cells(l)
                #diff-cells(r)
              ]
            }
            #html.tr(class: "diff-line")[
              #diff-cells((kind: "end", nr: 0, spans: ()))
              #diff-cells((kind: "end", nr: 0, spans: ()))
            ]
          ]
        ]
      ]
    }

    #html.div(class: "diff-scroll-padding")
  ]
]
