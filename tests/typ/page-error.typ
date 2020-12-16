// Test error cases of the `page` function.

// compare-ref: false
// error: 8:8-8:19 invalid paper
// error: 11:17-11:20 aligned axis

// Invalid paper.
[page: nonexistant]

// Aligned axes.
[page: main-dir=ltr]
