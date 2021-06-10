// Test stack layouts.

---
#let rect(width, color) = rect(width: width, height: 1cm, fill: color)
#stack(
    rect(2cm, #2a631a),
    rect(3cm, forest),
    rect(1cm, conifer),
)
