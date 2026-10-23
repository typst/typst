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
#context test(here().position(), none)
#html.frame(box(width: 100pt, height: 50pt, {
  place(top + left, dx: 30pt, dy: 20pt, context {
    test(here().position(), (x: 30pt, y: 20pt))
  })
}))

--- html-frame-shared-defs html ---
// Test that definitions shared by multiple frames are written only once, at
// the end of the body.
#let shared = {
  box(clip: true, fill: gradient.linear(red, blue))[A]
  box(fill: tiling(size: (4pt, 4pt), circle(radius: 1pt)))[B]
}
#html.frame(shared)
#html.frame(shared)

--- html-frame-without-body html ---
// Test a frame without being wrapped with a body.
#html.html(html.div(html.frame(box[hi])))

--- html-frame-with-dual-body html ---
// Test a frame with two body elements.
#html.html(
  html.body({
    html.div(html.elem("body", html.frame(box[hi])))
    html.frame(box[hi])
  }),
)
