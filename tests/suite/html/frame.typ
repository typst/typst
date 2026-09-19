--- html-frame html ---
A rectangle:
#html.frame(rect())

--- html-frame-in-layout paged ---
// Ensure that HTML frames are transparent in layout. This is less important for
// actual paged export than for _nested_ HTML frames, which take the same code
// path.
#html.frame[A]

--- html-frame-shared-defs html ---
// Test that definitions shared by multiple frames are written only once, at
// the end of the body.
#let shared = {
  box(clip: true, fill: gradient.linear(red, blue))[A]
  box(fill: tiling(size: (4pt, 4pt), circle(radius: 1pt)))[B]
}
#html.frame(shared)
#html.frame(shared)
