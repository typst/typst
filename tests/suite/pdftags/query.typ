--- query-tags-duplicate-heading pdftags ---
// This will display the heading with the same location a second time
#context query(heading).join()
= Hi

--- query-tags-duplicate-labelled-element pdftags ---
#figure(alt: "Text saying: hello there")[
  hello there
] <figure>

#context query(<figure>).at(0)

--- query-tags-ambigous-parent-place pdftags ---
// Error: 2-43 PDF/UA1 error: ambigous logical parent
// Hint: 2-43 please report this as a bug
#place(float: true, top + left)[something] <placed>

#context query(<placed>).join()

--- query-tags-ambigous-parent-footnote pdftags ---
// Error: 1:2-1:21 PDF/UA1 error: ambigous logical parent
// Hint: 1:2-1:21 please report this as a bug
#footnote[something] <note>

#context query(<note>).join()
