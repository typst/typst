// Ref: false

// Error: 9-21 cyclic import
#import "cycle2.typ" using *
#let inaccessible = "wow"

This is the first element of an import cycle.
