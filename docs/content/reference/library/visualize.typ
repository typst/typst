#import "../../../components/index.typ": docs-category

#show: docs-category.with(
  title: "Visualize",
  description: "Documentation for drawing and data visualization functionality.",
  category: "visualize",
)

Drawing and data visualization.

If you want to create more advanced drawings or plots, also have a look at the #link("https://github.com/johannes-wolf/cetz")[CeTZ] package as well as more specialized #link("https://typst.app/universe")[packages] for your use case.

= Accessibility <accessibility>
All shapes and paths drawn by Typst are automatically marked as @pdf.artifact[artifacts] to make them invisible to Assistive Technology (AT) during PDF export. However, their contents (if any) remain accessible.

If you are using the functions in this category to create an illustration with semantic meaning, make it accessible by wrapping it in a @figure function call. Use its @figure.alt[`alt` parameter] to provide an @guides:accessibility:textual-representations[alternative description].
