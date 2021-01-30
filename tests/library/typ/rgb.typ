// Check the output.
#[rgb 0.0, 0.3, 0.7]

// Alpha channel.
#[rgb 1.0, 0.0, 0.0, 0.5]

// Warning: 2:7-2:10 should be between 0.0 and 1.0
// Warning: 1:12-1:16 should be between 0.0 and 1.0
#[rgb -30, 15.5, 0.5]

// Error: 1:7-1:11 missing argument: blue component
#[rgb 0, 1]

// Error: 3:6-3:6 missing argument: red component
// Error: 2:6-2:6 missing argument: green component
// Error: 1:6-1:6 missing argument: blue component
#[rgb]
