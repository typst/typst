// Ref: false

// Error: cyclic import
#import "cycle1.typ": *
#let val = "much cycle"

This is the second element of an import cycle.
