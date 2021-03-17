// Test the base functions.
// Ref: false

---
#test(type("hi"), "string")
#test(repr([Hi #rect[there]]), "[Hi [<node box>]]")

---
// Check the output.
#test(rgb(0.0, 0.3, 0.7), #004db3)

// Alpha channel.
#test(rgb(1.0, 0.0, 0.0, 0.5), #ff000080)

// Warning: 2:11-2:14 should be between 0.0 and 1.0
// Warning: 1:16-1:20 should be between 0.0 and 1.0
#test(rgb(-30, 15.5, 0.5), #00ff80)

// Error: 11-15 missing argument: blue component
#test(rgb(0, 1), #00ff00)

// Error: 3:11-3:11 missing argument: red component
// Error: 2:11-2:11 missing argument: green component
// Error: 1:11-1:11 missing argument: blue component
#test(rgb(), #000000)
