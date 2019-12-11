[page.size: w=5cm, h=5cm]
[page.margins: 0cm]

// Test 1
[box: w=1, h=1, debug=false][
    [box][
        [align: center]
        [box: ps=3cm, ss=1cm]
        [direction: ttb, ltr]
        [box: ps=3cm, ss=1cm]
        [box: ps=1cm, ss=1cm]
        [box: ps=2cm, ss=1cm]
        [box: ps=1cm, ss=1cm]
    ]
]
[page.break]

// Test 2
[box: w=1, h=1, debug=false][
    [align: secondary=top] Top
    [align: secondary=center] Center
    [align: secondary=bottom] Bottom
    [direction: ttb, ltr]
    [align: secondary=origin, primary=bottom]
    [box: w=1cm, h=1cm]
]
[page.break]

// Test 3
[box: w=1, h=1, debug=false][
    [align: center][
        Somelongspacelessword!
        [align: left] Some
        [align: right] word!
    ]
]
[page.break]

// Test 4
[box: w=1, h=1, debug=false][
    [direction: ltr, ttb]
    [align: center]
    [align: secondary=origin]
    [box: ps=1cm, ss=1cm]
    [align: secondary=center]
    [box: ps=3cm, ss=1cm]
    [box: ps=4cm, ss=0.5cm]
    [align: secondary=end]
    [box: ps=2cm, ss=1cm]
]
[page.break]

// Test 5
[box: w=1, h=1, debug=false][
    [direction: primary=btt, secondary=ltr]
    [align: primary=center, secondary=left]
    [box: h=2cm, w=1cm]

    [direction: rtl, btt]
    [align: center]
    [align: vertical=origin] ORIGIN
    [align: vertical=center] CENTER
    [align: vertical=end] END
]
[page.break]

// Test 6
[box: w=1, h=1, debug=false][
    [box: w=4cm, h=1cm]

    [align: primary=right, secondary=center] CENTER

    [direction: btt, rtl]
    [align: primary=center, secondary=origin]
    [box: w=0.5cm, h=0.5cm]
    [box: w=0.5cm, h=1cm]
    [box: w=0.5cm, h=0.5cm]

    [align: primary=origin, secondary=end]
    END
]
