// Test unselectable text in PDF.

--- text-unselectable pdf pdftags ---

Selectable.

Unselectable numbers: foo #text(selectable: false)[000 123 *456* #super[_789_]] bar.

#text(selectable: false)[
  Unselectable paragraph spanning multiple lines.

  #pdf.artifact[No tagging in artifacts.]
]

*Bold* and *a#text(selectable: false)[B]c*.
