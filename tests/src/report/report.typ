#let conf(title: "", style: "", script: "", content) = {
  html.elem("html", attrs: ("lang": "en"))[
    #html.head[
      #html.meta(charset: "utf-8")
      #html.title[#title]
      #html.style(style)
    ]
    #html.body[
      #content
      #html.script(script)
    ]
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

#let text-diff(diff) = {
  html.table(class: "text-diff")[
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
}

#let image-diff(diff, n) = {
  html.div(class: "image-diff")[
    #html.div(class: "image-controls")[
      #html.fieldset(class: "header-control")[
        #html.legend[View mode]
        #html.label[
          #html.input(
            type: "radio",
            class: "image-view-mode",
            name: "image-view-mode-" + str(n),
            value: "side-by-side",
            checked: true,
          )
          side by side
        ]
        #html.label[
          #html.input(
            type: "radio",
            class: "image-view-mode",
            name: "image-view-mode-" + str(n),
            value: "fade",
          )
          fade
        ]
        #html.label[
          #html.input(
            type: "radio",
            class: "image-view-mode",
            name: "image-view-mode-" + str(n),
            value: "difference"
          )
          difference
        ]
      ]

      #html.fieldset(class: "header-control")[
        #html.label[
          #html.input(type: "range", class: "image-scale", min: 0.5, max: 8, value: 2, step: 0.05)
          scale
        ]
        #html.label[
          #html.input(type: "checkbox", class: "pixelated")
          pixelated
        ]
      ]
    ]

    #html.div(class: "image-diff-area")[
      #html.div(class: "image-diff-wrapper")[
        #html.div(class: "image-split")[
          #image(diff.left.data)
        ]
        #html.div(class: "image-split")[
          #image(diff.right.data)
        ]
      ]
    ]

    #html.div(class: "image-mode-controls")[
      #html.fieldset(class: "header-control")[
        #html.legend[Y-Align]
        #html.label[
          #html.input(
            type: "radio",
            class: "image-align-y",
            name: "image-align-y" + str(n),
            value: "top",
            checked: true,
          )
          top
        ]
        #html.label[
          #html.input(
            type: "radio",
            class: "image-align-y",
            name: "image-align-y" + str(n),
            value: "center",
          )
          center
        ]
        #html.label[
          #html.input(
            type: "radio",
            class: "image-align-y",
            name: "image-align-y" + str(n),
            value: "bottom",
          )
          bottom
        ]
      ]

      #html.fieldset(class: "header-control image-align-x-control")[
        #html.legend[X-Align]
        #html.label[
          #html.input(
            type: "radio",
            class: "image-align-x",
            name: "image-align-x" + str(n),
            value: "left",
            checked: true,
          )
          left
        ]
        #html.label[
          #html.input(
            type: "radio",
            class: "image-align-x",
            name: "image-align-x" + str(n),
            value: "center",
          )
          center
        ]
        #html.label[
          #html.input(
            type: "radio",
            class: "image-align-x",
            name: "image-align-x" + str(n),
            value: "right",
          )
          right
        ]
      ]

      #html.fieldset(class: "header-control image-blend-control")[
        #html.label[
          #html.input(type: "range", class: "image-blend", min: 0, max: 1, value: 0.5, step: 0.01)
          blend
        ]
      ]
    ]
  ]
}

#show: conf.with(
  title: "Typst test report",
  style: sys.inputs.style,
  script: sys.inputs.script,
)

#html.div(class: "container")[
  #html.div(class: "sidebar-container")[
    #html.div(class: "sidebar")[
      #html.ul(class: "sidebar-list")[
        #for report in sys.inputs.reports {
          html.li[
            #link("#" + report.name)[#report.name]
          ]
        }
      ]
    ]
  ]
  #html.div(class: "diff-container")[
    #let num-image-diffs = 0
    #for report in sys.inputs.reports {
      let first = true
      for diff in report.diffs {
        let content = [
          #html.details(open: true)[
            #html.summary(class: "diff-summary")[
              #html.h1(class: "diff-header")[
                #html.div(class: "diff-header-split")[
                  *#link("../../" + diff.left.path)[#diff.left.path]*
                ]
                #html.div(class: "diff-header-split")[
                  *#link("../../" + diff.right.path)[#diff.right.path]*
                ]
              ]
              #html.div(class: "diff-spacer")
            ]

            #if diff.kind == "text" {
              text-diff(diff)
            } else if diff.kind == "image" {
              num-image-diffs += 1
              image-diff(diff, num-image-diffs)
            } else {
              panic("unhandled diff kind: " + diff.kind)
            }
          ]
        ]

        // Add
        if first {
          html.div(class: "file-diff", id: report.name, content)
          first = false
        } else {
          html.div(class: "file-diff", content)
        }
      }
    }

    #html.div(class: "diff-scroll-padding")
  ]
]
