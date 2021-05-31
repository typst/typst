// Ref: false

// Error: 9-21 cyclic import
#import "cycle1.typ" using *
#let val = "much cycle"

This is the second element of an import cycle.
