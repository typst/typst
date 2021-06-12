// Test grid layouts.

---
#let rect(width, color) = rect(width: width, height: 2cm, fill: color)

#page(width: 100pt, height: 140pt)
#grid(
    columns: (auto, 1fr, 3fr, 0.25cm, 3%, 2mm + 10%),
    rect(0.5cm, #2a631a),
    rect(100%, forest),
    rect(100%, conifer),
    rect(100%, #ff0000),
    rect(100%, #00ff00),
    rect(80%, #00faf0),
    rect(1cm, #00ff00),
    rect(0.5cm, #2a631a),
    rect(100%, forest),
    rect(100%, conifer),
    rect(100%, #ff0000),
    rect(100%, #00ff00),
)

#grid()

---
#grid(
    columns: (auto, auto, 40%),
    gutter-columns: (1fr,),
    gutter-rows: (1fr,),
    rect(fill: eastern)[dddaa aaa aaa],
    rect(fill: conifer)[ccc],
    rect(width: 100%, fill: #dddddd)[aaa],
)

---
#page(height: 3cm, width: 2cm)
#grid(
    columns: (1fr, 1cm, 1fr, 1fr),
    column-dir: ttb,
    rows: (auto, 1fr),
    rect(height: 100%, fill: #222222)[foo],
    rect(height: 100%, fill: #547d0a)[bar],
    rect(height: 100%, fill: eastern)[hab],
    rect(height: 100%, fill: conifer)[baz],
    rect(height: 100%, width: 100%, fill: #547d0a)[bar],
)

---
#page(height: 3cm, margins: 0pt)
#align(center)
#grid(
    columns: (1fr,),
    rows: (1fr, auto, 2fr),
    [], rect(width: 100%)[A bit more to the top], [],
)

