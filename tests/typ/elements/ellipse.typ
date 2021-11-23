// Test the `ellipse` function.

---
// Default ellipse.
#ellipse()

---
Rect in ellipse in fixed rect. \
#rect(width: 3cm, height: 2cm, fill: rgb("2a631a"),
  ellipse(fill: forest,
    rect(fill: conifer,
      align(center, horizon)[
        Stuff inside an ellipse!
      ]
    )
  )
)

Auto-sized ellipse. \
#ellipse(fill: conifer, stroke: forest, thickness: 3pt, padding: 3pt)[
  #font(8pt)
  But, soft! what light through yonder window breaks?
]
