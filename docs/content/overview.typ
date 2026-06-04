#import "../components/index.typ": (
  big-nav-button, def-dest, docs-chapter, icon, insertion,
)

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

Welcome to Typst's documentation! Typst is a markup-based typesetting system that combines powerful automation and high-quality typography with speed and ease of use. This makes it suitable for documents of any complexity. Typst is a great alternative to both word processors and LaTeX.

This documentation is split into multiple parts, serving different needs:

- If you are new to Typst, we highly recommend starting with our @tutorial[beginner-friendly tutorial]. Throughout the tutorial, we will introduce you to Typst through a practical example.

- To answer targeted questions about Typst and familiarize yourself with advanced features, use the @reference[reference]. It describes the fundamental features of the Typst language and contains sections for all the functions, types, and more that come with Typst.

- For tailored, in-depth how-tos on specific features, use cases, and audiences, check out our @guides[guides]. They provide copyable snippets throughout and allow you to build confidence with a specific feature area. If you are coming from LaTeX, the @guides:for-latex-users provides an alternative introduction to Typst, building on concepts you already know.

The term _Typst_ refers to three concepts: The Typst language, the Typst compiler, and the Typst web app. The language is what you write, the compiler translates files in the Typst language into PDFs, HTML pages, and other formats, and the Typst web app lets you work collaboratively on Typst projects in your browser. The Typst language and the compiler are open-source.

This documentation primarily documents the Typst language, although the tutorial and various pages will refer to the web app and the command line Typst compiler.
#insertion(
  "overview-web-app",
  fallback: [
    In the copy of the docs hosted on #link("https://typst.app/docs/"), we also include documentation about the web app.
  ],
)
To learn how to install the Typst compiler CLI, visit the #link("https://typst.app/open-source")[Open Source page] on our website. There, you can also learn more about the relationship between the Typst compiler and the web app. Once you have installed the Typst compiler CLI, run `typst help` for more information on how to use it.

Our #link("https://github.com/typst/typst")[GitHub repository] provides additional developer-facing documentation about how to contribute to Typst and how to integrate it into your applications.

The documentation also contains a @changelog[changelog], in which you can track the evolution of Typst and what changes to the markup language mean for your projects. This documentation applies to Typst #sys.version.
