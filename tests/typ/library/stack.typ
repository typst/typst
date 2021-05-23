// Test the `stack` function.

---
#let rect(width, color) = rect(width: width, height: 1cm, fill: color)
#stack(
    rect(2cm, #2a631a),
    rect(3cm, #43a127),
    rect(1cm, #9feb52),
)
