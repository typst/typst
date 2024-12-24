// Test numbering styles

--- numbering-shorthand ---
#test(numbering("1", 1, 2, 3, 4, 5, 6, 7, 8, 9, 10), "12345678910")

--- numbering-shorthand-prefix ---
#test(numbering("p1", 1, 2, 3, 4, 5, 6, 7, 8, 9, 10), "p1p2p3p4p5p6p7p8p9p10")

--- numbering-verbose-prefix ---
#test(numbering("prefix{decimal}", 1, 2, 3), "prefix1prefix2prefix3")

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
