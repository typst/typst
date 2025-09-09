--- query-tags-duplicate-heading pdftags ---
// This will display the heading with the same location a second time
#context query(heading).join()
= Hi

--- query-tags-duplicate-labelled-element pdftags ---
#figure[
  hello there
] <figure>

#context query(<figure>).at(0)

--- query-tags-ambigous-parent-place pdftags ---
// Error: 2-43 PDF/UA1 error: ambigous logical parent
// Hint: 2-43 please report this as a bug
#place(float: true, top + left)[something] <placed>

#context query(<placed>).join()

--- query-tags-ambigous-parent-footnote pdftags ---
// TODO: add test once tag nesting is fixed
#footnote[something] <note>

#context query(<note>).join()
