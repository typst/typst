// Test error cases of the `page` function.

// Invalid paper.
[page: nonexistant]

// Aligned axes.
[page: main-dir=ltr]

// compare-ref: false
// error: 4:8-4:19 invalid paper
// error: 7:17-7:20 aligned axis
