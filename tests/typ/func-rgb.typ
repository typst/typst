// Test the `rgb` function.

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

// warning: 10:6-10:9 must be between 0.0 and 1.0
// warning: 10:11-10:15 must be between 0.0 and 1.0
// error: 13:6-13:10 missing argument: blue component
// error: 16:5-16:5 missing argument: red component
// error: 16:5-16:5 missing argument: green component
// error: 16:5-16:5 missing argument: blue component
