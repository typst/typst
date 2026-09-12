--- html-frame html ---
A rectangle:
#html.frame(rect())

--- html-frame-in-layout paged ---
// Ensure that HTML frames are transparent in layout. This is less important for
// actual paged export than for _nested_ HTML frames, which take the same code
// path.
#html.frame[A]

--- html-frame-position html ---
// Test that positions are available within a frame, but nowhere else.
#context test(here().position(), (page: 1, x: none, y: none))
#html.frame(box(width: 100pt, height: 50pt, {
  place(top + left, dx: 30pt, dy: 20pt, context {
    test(here().position(), (page: 1, x: 30pt, y: 20pt))
  })
}))
