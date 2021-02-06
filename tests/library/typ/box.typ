#[page "a5", flip: true]

// Rectangle with width, should have paragraph height
#[box width: 2cm, color: #9650D6][aa]

Sometimes there is no box

// Rectangle with height, should span line
#[box height: 2cm, width: 100%, color: #734CED][bb]

// Empty rectangle with width and height
#[box width: 6cm, height: 12pt, color: #CB4CED]

// This empty rectangle should not be displayed
#[box width: 2in, color: #ff0000]

// This one should be
#[box height: 15mm, width: 100%, color: #494DE3]

// These are in a row!
#[box width: 2in, height: 10pt, color: #D6CD67]
#[box width: 2in, height: 10pt, color: #EDD466]
#[box width: 2in, height: 10pt, color: #E3BE62]
