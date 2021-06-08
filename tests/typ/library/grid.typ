// Test the `grid` function.

---
#page(width: 100pt, height: 140pt)
#let rect(width, color) = rect(width: width, height: 2cm, fill: color)
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
    gutter: (1fr,),
    rect(fill: eastern)[dddaa aaa aaa],
    rect(fill: conifer)[ccc],
    rect(width: 100%, fill: #dddddd)[aaa],
)

#grid(
    columns: (auto, auto, 40%),
    gutter: (1fr,),
    rect(fill: eastern)[dddaa aaa aaa],
    rect(fill: conifer)[ccc],
    rect(width: 100%, fill: #dddddd)[aaa],
)


---

#page(width: 12cm, height: 2.5cm)
#grid(
    columns: (auto, auto, auto, auto, auto),
    gutter-col: (2fr, 1fr, 1fr),
    gutter-row: (6pt, 6pt, 6pt, 6pt),
    [*Quarter*],
    [Expenditure],
    [External Revenue],
    [Financial ROI],
    [_total_],
    [*Q1*],
    [173,472.57 \$],
    [472,860.91 \$],
    [51,286.84 \$],
    [_350,675.18 \$_],
    [*Q2*],
    [93,382.12 \$],
    [439,382.85 \$],
    [-1,134.30 \$],
    [_344,866.43 \$_],
    [*Q3*],
    [96,421.49 \$],
    [238,583.54 \$],
    [3,497.12 \$],
    [_145,659.17 \$_],
)

---
#page(height: 3cm, width: 2cm)
#grid(
    dir: ttb,
    columns: (1fr, 1cm, 1fr, 1fr),
    rows: (auto, 1fr),
    rect(height: 100%, fill: #222222)[foo],
    rect(height: 100%, fill: #547d0a)[bar],
    rect(height: 100%, fill: eastern)[hab],
    rect(height: 100%, fill: conifer)[baz],
    rect(height: 100%, width: 100%, fill: #547d0a)[bar],
)
