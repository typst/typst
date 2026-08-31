--- html-frame html ---
A rectangle:
#html.frame(rect())

--- html-frame-par html ---
A #html.frame(rect()) B.

--- html-frame-baseline-shift html ---
A #html.frame(box(baseline: 1em, rect(width: 1em, height: 1em))) B.

--- html-frame-in-layout paged ---
// Ensure that HTML frames are transparent in layout. This is less important for
// actual paged export than for _nested_ HTML frames, which take the same code
// path.
#html.frame[A]
