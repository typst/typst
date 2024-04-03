// Test shape fill & stroke for specific values that used to make the stroke deformed.

#set page(
  width: 8cm,
  height: 8cm
)

== Stroke behaves as expected
// Stroke behaves as expected for these values

// Rectangle with stroke of length 15.9mm
#rect(
  radius: 1mm,
  stroke: (left: rgb("46b3c2")+15.9mm)
)[
  Text. (Rectangle)
]

// Block with stroke of length 16.1mm
#block(
  radius: 1mm,
  stroke: (left: rgb("9650d6")+16.1mm)
)[
  Text. (Block)
]
\

== Stroke got deformed
// Stroke used to get deformed for a number of values (these are just two)

// Rectangle with stroke of length 16.0mm
#rect(
  radius: 1mm,
  stroke: (left: rgb("46b3c2")+16.0mm)
)[
  Text. (Rectangle)
]

// Block with stroke of length 16.0mm
#block(
  radius: 1mm,
  stroke: (left: rgb("9650d6")+16.0mm)
)[
  Text. (Block)
]

// Block with stroke of length 18.2mm
#block(
  radius: 1mm,
  stroke: (left: rgb("9650d6")+18.2mm)
)[
  Text. (Block)
]
