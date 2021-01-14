// Automatically initialized with `none`.
#let x;
{(x,)}

// Can start with underscore.
#let _y=1;
{_y}

// Multiline.
#let z = "world"
    + " 🌏"; Hello, {z}!

// Error: 1:6-1:7 expected identifier, found integer
#let 1;

// Error: 4:1-4:3 unexpected identifier
// Error: 3:4-3:9 unexpected identifier
// Error: 3:1-3:1 expected semicolon
#let x = ""
Hi there
