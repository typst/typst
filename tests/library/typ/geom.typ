#[page "a5", flip: true]

// Rectangle with width, should have paragraph height
#[rect width: 2cm, color: #9650D6][aa]

Sometimes there is no box

// Rectangle with height, should span line
#[rect height: 2cm, color: #734CED][bb]

// Empty rectangle with width and height
#[rect width: 6cm, height: 12pt, color: #CB4CED]

// This empty rectangle should not be displayed
#[rect width: 2in, color: #ff0000]

// This one should be
#[rect height: 15mm, color: #494DE3]

// These are in a row!
#[rect width: 2in, height: 10pt, color: #D6CD67]
#[rect width: 2in, height: 10pt, color: #EDD466]
#[rect width: 2in, height: 10pt, color: #E3BE62]
