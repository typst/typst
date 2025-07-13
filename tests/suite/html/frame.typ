// No proper HTML tests here yet because we don't want to test SVG export just
// yet. We'll definitely add tests at some point.

--- html-frame-in-layout ---
// Ensure that HTML frames are transparent in layout. This is less important for
// actual paged export than for _nested_ HTML frames, which take the same code
// path.
#html.frame[A]
