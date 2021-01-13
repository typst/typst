// Test the `rgb` function.

// Check the output.
[rgb 0.0, 0.3, 0.7]

// Alpha channel.
[rgb 1.0, 0.0, 0.0, 0.5]

// Warning: 2:6-2:9 must be between 0.0 and 1.0
// Warning: 1:11-1:15 must be between 0.0 and 1.0
[rgb -30, 15.5, 0.5]

// Error: 1:6-1:10 missing argument: blue component
[rgb 0, 1]

// Error: 3:5-3:5 missing argument: red component
// Error: 2:5-2:5 missing argument: green component
// Error: 1:5-1:5 missing argument: blue component
[rgb]
