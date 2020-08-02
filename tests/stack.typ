[page: w=5cm, h=5cm, margins=0cm]

// Test 1
[box: w=1, h=1, debug=false][
    [box][
        [align: center]
        [box: ps=3cm, ss=1cm]
        [direction: p=ttb, s=ltr]
        [box: ps=3cm, ss=1cm]
        [box: ps=1cm, ss=1cm]
        [box: ps=2cm, ss=1cm]
        [box: ps=1cm, ss=1cm]
    ]
]

// Test 2
[box: w=1, h=1, debug=false][
    [align: s=top] Top
    [align: s=center] Center
    [align: s=bottom] Bottom
    [direction: p=ttb, s=ltr]
    [align: p=bottom, s=origin]
    [box: w=1cm, h=1cm]
]

// Test 3
[box: w=1, h=1, debug=false][
    [align: center][
        Some-long-spaceless-word!
        [align: left] Some
        [align: right] word!
    ]
]

// Test 4
[box: w=1, h=1, debug=false][
    [direction: p=ltr, s=ttb]
    [align: center]
    [align: s=origin] [box: ps=1cm, ss=1cm]
    [align: s=center] [box: ps=3cm, ss=1cm] [box: ps=4cm, ss=0.5cm]
    [align: s=end] [box: ps=2cm, ss=1cm]
]

// Test 5
[box: w=1, h=1, debug=false][
    [direction: p=btt, s=ltr]
    [align: p=center, s=left]
    [box: h=2cm, w=1cm]

    [direction: p=rtl, s=btt]
    [align: center]
    [align: v=origin] ORIGIN
    [align: v=center] CENTER
    [align: v=end] END
]

// Test 6
[box: w=1, h=1, debug=false][
    [box: w=4cm, h=1cm]

    [align: p=right, s=center] CENTER

    [direction: p=btt, s=rtl]
    [align: p=center, s=origin]
    [box: w=0.5cm, h=0.5cm]
    [box: w=0.5cm, h=1cm]
    [box: w=0.5cm, h=0.5cm]

    [align: p=origin, s=end] END
]
