--- html-frame html ---
A rectangle:
#html.frame(rect())

--- html-frame-in-layout ---
// Ensure that HTML frames are transparent in layout. This is less important for
// actual paged export than for _nested_ HTML frames, which take the same code
// path.
#html.frame[A]
