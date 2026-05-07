#import "../components/index.typ": big-nav-button, def-dest, docs-chapter, icon

#show: docs-chapter.with(
  title: "Overview",
  route: "/",
  def-target: <overview>,
  description: "Learn how to use Typst to compose documents faster. Get started with the tutorial, or dive into the reference.",
  nav-buttons: html.div(class: "doc-categories", {
    context big-nav-button(
      icon: icon(32, "tutorial-c", "Circled play icon"),
      href: def-dest(<tutorial>),
      title: "Tutorial",
      description: [Step-by-step guide to help you get started.],
    )
    context big-nav-button(
      icon: icon(32, "reference-c", "Circled information icon"),
      href: def-dest(<reference>),
      title: "Reference",
      description: [Details about all syntax, concepts, types, and functions.],
    )
  }),
)

Welcome to Typst's documentation! Typst is a new markup-based typesetting system for the sciences. It is designed to be an alternative both to advanced tools like LaTeX and simpler tools like Word and Google Docs. Our goal with Typst is to build a typesetting tool that is highly capable _and_ a pleasure to use.

This documentation is split into two parts: A beginner-friendly tutorial that introduces Typst through a practical use case and a comprehensive reference that explains all of Typst's concepts and features.

We also invite you to join the community we're building around Typst. Typst is still a very young project, so your feedback is more than valuable.
