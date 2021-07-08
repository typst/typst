// Ref: false

// Error: 16-28 cyclic import
#import * from "cycle1.typ"
#let val = "much cycle"

This is the second element of an import cycle.
