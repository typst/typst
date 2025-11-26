#let icons = (
  view-side-by-side: "M7.43 5.438v6.568h1.25V5.438ZM3.744 3.047a1 1 0 0 0-1 1v7.92a1 1 0 0 0 1 1h8.512a1 1 0 0 0 1-1v-7.92a1 1 0 0 0-1-1H3.869zm.942.857a.685.685 0 1 1 0 1.371.685.685 0 0 1 0-1.37M7.43 5.438h1.25v.609h3.326v5.67H8.68v.289H7.43v-.29H3.994v-5.67H7.43z",
  view-blend: "M4.086 8.12 1.158 9.183a.2.2 0 0 0-.025.365l6.805 3.617c.25.133.544.153.81.057l5.533-2.012a.199.199 0 0 0 .026-.363l-2.952-1.57-2.503.91a1.25 1.25 0 0 1-1.014-.073zm3.203-5.403a1 1 0 0 0-.307.06L1.027 4.941l7.2 3.83a1 1 0 0 0 .812.057l5.953-2.166-7.199-3.83a1 1 0 0 0-.504-.115m.02 1.217 4.744 2.523-3.34 1.215L3.97 5.148Z",
  view-difference: "M5 4a4 4 0 1 0 1.693 7.625c.14-.065.142-.258.016-.346A4 4 0 0 1 5 8c0-1.357.676-2.556 1.709-3.28.126-.088.124-.28-.016-.345A4 4 0 0 0 5 4m6 0a4.01 4.01 0 0 0-4 4c0 2.202 1.798 4 4 4s4-1.798 4-4-1.798-4-4-4m0 1.2c1.554 0 2.8 1.246 2.8 2.8s-1.246 2.8-2.8 2.8A2.79 2.79 0 0 1 8.2 8c0-1.554 1.246-2.8 2.8-2.8",

  align-top: "M2 3.016v1.25h12v-1.25zm6 3-.443.441-2 2-.442.441.885.885.441-.441.934-.934v4.582h1.25V8.408l.932.934.443.441.883-.885-.442-.441-2-2Z",
  align-horizon: "M7.375 10.174v4.857h1.25v-4.857zM8 9.504l-2.441 2.441-.444.444.885.882.441-.44.934-.935v-1.722h1.25v1.722l.934.934.441.441.883-.882-.442-.444Zm-.625-8.486v3.134l-.934-.931L6 2.779l-.885.883.444.443 2 2L8 6.547l.441-.442 2.002-2 .442-.443L10 2.78l-.441.442-.934.931V1.018ZM2 7.75V9h12V7.75Z",
  align-bottom: "M2 11.75V13h12v-1.25Zm5.375-8.775V7.64l-.932-.934L6 6.266l-.883.884.442.442 2 2 .441.441.443-.441 2-2 .442-.442L10 6.266l-.441.441-.934.934V2.975Z",

  align-left: "M3.38 2v12h1.25V2ZM2 9c-.554 0-1 .446-1 1v1c0 .554.446 1 1 1h1.38V9Zm2.63 0v3H6c.554 0 1-.446 1-1v-1c0-.554-.446-1-1-1ZM2 4c-.554 0-1 .446-1 1v1c0 .554.446 1 1 1h1.38V4Zm2.63 0v3H12c.554 0 1-.446 1-1V5c0-.554-.446-1-1-1Z",
  align-center: "M7.375 2v12h1.25V2ZM6 9c-.554 0-1 .446-1 1v1c0 .554.446 1 1 1h1.375V9Zm2.625 0v3H10c.554 0 1-.446 1-1v-1c0-.554-.446-1-1-1ZM3 4c-.554 0-1 .446-1 1v1c0 .554.446 1 1 1h4.375V4Zm5.625 0v3H13c.554 0 1-.446 1-1V5c0-.554-.446-1-1-1Z",
  align-right: "M11.375 2v12h1.25V2ZM10 9c-.554 0-1 .446-1 1v1c0 .554.446 1 1 1h1.375V9Zm2.625 0v3H14c.554 0 1-.446 1-1v-1c0-.554-.446-1-1-1ZM4 4c-.554 0-1 .446-1 1v1c0 .554.446 1 1 1h7.375V4Zm8.625 0v3H14c.554 0 1-.446 1-1V5c0-.554-.446-1-1-1Z",

  antialiasing: "M4 11v3h3v-3ZM2 8v3h3V8Zm0-3v3h3V5Zm2-3v3h3V2Zm5 .084v1.273a4.75 4.75 0 0 1 0 9.286v1.271a5.999 5.999 0 0 0 0-11.83M7.375 1v14h1.25V1Z",

  plus: "M7.375 2v5.375H2v1.25h5.375V14h1.25V8.625H14v-1.25H8.625V2Z",
  minus: "M2 7.375v1.25h12v-1.25z",
)

#let svg-icon(path) = {
  html.elem(
    "svg",
    attrs: (
      xmlns: "http://www.w3.org/2000/svg",
      width: "16",
      height: "16",
      fill: "currentColor",
    ),
  )[
    #html.elem("path", attrs: (d: path))
  ]
}

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
  let radio-icon-button(name: none, value: none, title: none, icon: none, checked: false) = {
    html.label(class: "icon-toggle-button")[
      #html.input(
        type: "radio",
        class: name,
        name: name + str(n),
        title: title,
        value: value,
        checked: checked,
      )
      #svg-icon(icon)
    ]
  }

  let checkbox-icon-button(name: none, title: none, icon: none, checked: false) = {
    html.label(class: "icon-toggle-button")[
      #html.input(
        type: "checkbox",
        class: name,
        title: title,
        checked: checked,
      )
      #svg-icon(icon)
    ]
  }

  let icon-button(name: none, title: none, icon: none) = {
    html.button(class: "icon-button " + name, title: title)[
      #svg-icon(icon)
    ]
  }

  let slider(name: none, title: none, icon: none, min: 0, max: 1, value: 0.5, step: 0.01) = {
    html.label(class: "slider", title: title)[
      #if icon != none {
        svg-icon(icon)
      }
      #html.input(
        type: "range",
        class: name,
        min: min,
        max: max,
        value: value,
        step: step,
      )
    ]
  }

  html.div(class: "image-diff")[
    #html.div(class: "image-controls")[
      #html.fieldset(class: "control-group")[
        #radio-icon-button(
          name: "image-view-mode",
          value: "side-by-side",
          title: "View-Mode side by side",
          icon: icons.view-side-by-side,
          checked: true,
        )
        #radio-icon-button(
          name: "image-view-mode",
          value: "blend",
          title: "View-Mode blend",
          icon: icons.view-blend,
        )
        #radio-icon-button(
          name: "image-view-mode",
          value: "difference",
          title: "View-Mode difference",
          icon: icons.view-difference,
        )
      ]

      #html.fieldset(class: "control-group")[
        #checkbox-icon-button(
          name: "antialiasing",
          title: "Antialiasing",
          icon: icons.antialiasing,
          checked: true,
        )
      ]

      #html.fieldset(class: "control-group")[
        #icon-button(name: "image-zoom-minus", title: "Zoom out", icon: icons.minus)
        #icon-button(name: "image-zoom-plus", title: "Zoom in", icon: icons.plus)
        #slider(
          name: "image-zoom",
          min: 0.5, max: 8, value: 2, step: 0.05,
          title: "Zoom",
        )
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
      #html.fieldset(class: "control-group")[
        #radio-icon-button(
          name: "image-align-y",
          value: "top",
          title: "Vertical-align top",
          icon: icons.align-top,
          checked: true,
        )
        #radio-icon-button(
          name: "image-align-y",
          value: "center",
          title: "Vertical-align center",
          icon: icons.align-horizon
        )
        #radio-icon-button(
          name: "image-align-y",
          value: "bottom",
          title: "Vertical-align bottom",
          icon: icons.align-bottom
        )
      ]

      #html.fieldset(class: "control-group image-align-x-control")[
        #radio-icon-button(
          name: "image-align-x",
          value: "left",
          title: "Horizontal-align left",
          icon: icons.align-left,
          checked: true,
        )
        #radio-icon-button(
          name: "image-align-x",
          value: "center",
          title: "Horizontal-align center",
          icon: icons.align-center
        )
        #radio-icon-button(
          name: "image-align-x",
          value: "right",
          title: "Horizontal-align right",
          icon: icons.align-right
        )
      ]

      #html.fieldset(class: "control-group image-blend-control")[
        #slider(
          name: "image-blend",
          min: 0, max: 1, value: 0.5, step: 0.01,
          title: "Blend",
          icon: icons.view-blend,
        )
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
