// Test configuring the size and fitting behaviour of images.

// Fit to width of page.
[image "res/rhino.png"]

// Fit to height of page.
[page width: 270pt][
    [image "res/rhino.png"]
]

// Set width explicitly.
[image "res/rhino.png", width: 50pt]

// Set height explicitly.
[image "res/rhino.png", height: 50pt]

// Set width and height explicitly and force stretching.
[image "res/rhino.png", width: 25pt, height: 50pt]

// Make sure the bounding-box of the image is correct.
[align bottom, right][
    [image "res/tiger.jpg"]
]
