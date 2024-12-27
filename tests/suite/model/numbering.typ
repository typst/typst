// Test numbering styles

--- numbering-shorthand ---
#test(numbering("1", 1, 2, 3, 4, 5, 6, 7, 8, 9, 10), "12345678910")

--- numbering-shorthand-prefix ---
#test(numbering("p1", 1, 2, 3), "p1p2p3")

--- numbering-shorthand-prefix-suffix ---
#test(numbering("p1.a.is", 1, 3, 5), "p1.c.vs")

--- numbering-verbose-prefix ---
#test(numbering("prefix{decimal}", 1, 2, 3), "prefix1prefix2prefix3")

--- numbering-verbose-prefix-suffix ---
#test(numbering("prefix{circled-decimal}.{double-circled-decimal}.{filled-circled-decimal}suffix", 1, 1, 1), "prefix①.⓵.❶suffix")

--- numbering-additive ---
#test(numbering("{greek-upper-modern}", 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10), "𐆊ΑΒΓΔΕΣΤΖΗΘΙ")

--- numbering-fixed ---
#test(numbering("{double-circled-decimal}", 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11), "⓵⓶⓷⓸⓹⓺⓻⓼⓽⓾11")

--- numbering-numeric ---
#test(numbering("{decimal}", 1, 2, 3, 4, 5, 6, 7, 8, 9, 10), "12345678910")

--- numbering-symbolic ---
#test(numbering("{symbol}", 1, 2, 3, 4, 5, 6, 7, 8, 9, 10), "*†‡§¶‖**††‡‡§§")

--- numbering-no-name ---
// Error: 12-20 invalid numbering pattern
#numbering("{nope}", 1)

--- numbering-unclosed ---
// Error: 12-21 invalid numbering pattern
#numbering("{roman{", 1)

--- numbering-negative ---
// Error: 17-19 number must be at least zero
#numbering("1", -1)
