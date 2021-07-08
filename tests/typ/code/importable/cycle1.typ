// Ref: false

// Error: 16-28 cyclic import
#import * from "cycle2.typ"
#let inaccessible = "wow"

This is the first element of an import cycle.
