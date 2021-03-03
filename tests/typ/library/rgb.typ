// Test the rgb function.
// Ref: false

---
{
    // Check the output.
    test(rgb(0.0, 0.3, 0.7), #004db3)

    // Alpha channel.
    test(rgb(1.0, 0.0, 0.0, 0.5), #ff000080)

    // Warning: 2:14-2:17 should be between 0.0 and 1.0
    // Warning: 1:19-1:23 should be between 0.0 and 1.0
    test(rgb(-30, 15.5, 0.5), #00ff80)

    // Error: 14-18 missing argument: blue component
    test(rgb(0, 1), #00ff00)

    // Error: 3:14-3:14 missing argument: red component
    // Error: 2:14-2:14 missing argument: green component
    // Error: 1:14-1:14 missing argument: blue component
    test(rgb(), #000000)
}
