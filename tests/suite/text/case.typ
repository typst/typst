// Test the `upper` and `lower` functions.

--- lower-and-upper paged ---
#let memes = "ArE mEmEs gReAt?";
#test(lower(memes), "are memes great?")
#test(upper(memes), "ARE MEMES GREAT?")
#test(upper("Ελλάδα"), "ΕΛΛΆΔΑ")

--- cases-content-text paged ---
// Check that cases are applied to text nested in content
#lower(box("HI!"))

--- cases-content-symbol paged ---
// Check that cases are applied to symbols nested in content
#lower($H I !$.body)

--- cases-content-html html ---
#lower[MY #html.strong[Lower] #symbol("A")] \
#upper[my #html.strong[Upper] #symbol("a")] \

--- upper-bad-type paged ---
// Error: 8-9 expected string or content, found integer
#upper(1)
