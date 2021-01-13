// Test the `rgb` function.
//
// warning: 9:6-9:9 must be between 0.0 and 1.0
// warning: 9:11-9:15 must be between 0.0 and 1.0
// error: 12:6-12:10 missing argument: blue component
// error: 15:5-15:5 missing argument: red component
// error: 15:5-15:5 missing argument: green component
// error: 15:5-15:5 missing argument: blue component

// Check the output.
[rgb 0.0, 0.3, 0.7]

// Alpha channel.
[rgb 1.0, 0.0, 0.0, 0.5]

// Value smaller than 0.0 and larger than 1.0
[rgb -30, 15.5, 0.5]

// Missing blue component.
[rgb 0, 1]

// Missing all components.
[rgb]
