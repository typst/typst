// Test all gradient presets.

---
#set page(width: 200pt, height: auto, margin: 0pt)
#set text(fill: white, size: 18pt)
#set text(top-edge: "bounds", bottom-edge: "bounds")

#let presets = (
  ("turbo", color.map.turbo),
  ("cividis", color.map.cividis),
  ("rainbow", color.map.rainbow),
  ("spectral", color.map.spectral),
  ("viridis", color.map.viridis),
  ("inferno", color.map.inferno),
  ("magma", color.map.magma),
  ("plasma", color.map.plasma),
  ("rocket", color.map.rocket),
  ("mako", color.map.mako),
  ("vlag", color.map.vlag),
  ("icefire", color.map.icefire),
  ("flare", color.map.flare),
  ("crest", color.map.crest),
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
