--- list-tags-basic pdftags ---
- a
  - 1
- b
  - c
    - d
  - e
- f

--- list-tags-mixed-with-enum pdftags ---
- a
  + 1
- b
  + c
    - d
  + e
- f

--- list-tags-complex-item-with-sub-list pdftags ---
- #[#figure[hi] #footnote[1].]
  - a
  - b
- c
- d

--- list-tags-complex-item-with-nested-list pdftags ---
- #[
    #figure[hi]
    #footnote[1].
    - a
    - b
  ]
- c
- d
