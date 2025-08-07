--- lang-tags-pars-basic pdftags ---
#set text(lang: "uk")
Par 1.

#set text(lang: "sr")
Par 2.

#set text(lang: "be")
Par 3.

--- lang-tags-propagation pdftags ---
#set text(lang: "nl")
A paragraph.

// language attributes are propagated to the parent (L) tag
- #text(lang: "de", "a")
  - #text(lang: "de", "b")
  - #text(lang: "de", "c")
- #text(lang: "de", "d")
