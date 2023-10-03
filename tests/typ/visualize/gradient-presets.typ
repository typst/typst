// Test all gradient presets.

---
#set page(width: 200pt, height: auto, margin: 0pt)
#set text(fill: white, size: 18pt)
#set text(top-edge: "bounds", bottom-edge: "bounds")

#let presets = (
  ("turbo", gradient.turbo()),
  ("cividis", gradient.cividis()),
  ("rainbow", gradient.rainbow()),
  ("spectral", gradient.spectral),
  ("viridis", gradient.viridis),
  ("inferno", gradient.inferno),
  ("magma", gradient.magma),
  ("plasma", gradient.plasma),
  ("rocket", gradient.rocket),
  ("mako", gradient.mako),
  ("vlag", gradient.vlag),
  ("icefire", gradient.icefire),
  ("flare", gradient.flare),
  ("crest", gradient.crest),
)

#stack(
  spacing: 3pt,
  ..presets.map(((name, preset)) => block(
    width: 100%,
    height: 20pt,
    fill: gradient.linear(..preset),
    align(center + horizon, smallcaps(name)),
  ))
)
