--- svg-relative-paths ---
#block[
  #rect(width: 10pt, height: 10pt)
  #block(inset: 10pt)[
    #rect(width: 10pt, height: 10pt)
    #rotate(45deg,
      block(inset: 10pt)[
        #block(inset: 10pt)[
          #rect(width: 10pt, height: 10pt)
          #text("Hello world")
          #rect(width: 10pt, height: 10pt, radius: 10pt)
          #rotate(45deg,
            block(inset: 10pt)[
              #rect(width: 10pt, height: 10pt, radius: 10pt)
              #rect(width: 10pt, height: 10pt, radius: 10pt)
            ]
          )
        ]
      ]
    )
  ]
]
