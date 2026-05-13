#import "../../components/index.typ": (
  docs-chapter, paged-heading-offset, section-outline,
)

#docs-chapter(
  title: "Guides",
  route: "/guides",
  description: "Guides for Typst.",
)[
  Welcome to the Guides section! Here, you'll find helpful material for specific user groups or use cases. Please see the list below for the available guides. Feel free to propose other topics for guides!

  #section-outline(
    title: [List of Guides],
    label: <list-of-guides>,
  )
]

#show: paged-heading-offset.with(1)
#include "for-latex-users.typ"
#include "page-setup.typ"
#include "tables.typ"
#include "accessibility.typ"
