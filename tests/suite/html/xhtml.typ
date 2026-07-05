--- xhtml-serialization xhtml ---
#set document(title: "X")

#html.elem("HTML", attrs: (LANG: "en"))[
  #html.elem("BODY")[
    #html.elem("INPUT", attrs: (CHECKED: ""))
    #html.elem("SCRIPT")[if (a > b) console.log(a);]
    $ a #h(1em) b $
  ]
]
